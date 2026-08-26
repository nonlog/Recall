use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use walkdir::WalkDir;

use crate::adapters::{self, ResumeCommand};
use crate::db::store::Store;
use crate::types::Session;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeleteMode {
    Trash,
    Permanent,
    IndexOnly,
}

impl DeleteMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            DeleteMode::Trash => "trash",
            DeleteMode::Permanent => "permanent",
            DeleteMode::IndexOnly => "index-only",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DeletePlan {
    pub(crate) mode: DeleteMode,
    pub(crate) native_roots: Vec<PathBuf>,
    pub(crate) native_command: Option<ResumeCommand>,
    pub(crate) suppress_reindex: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeleteResult {
    pub(crate) mode: &'static str,
    pub(crate) deleted_from_index: bool,
    pub(crate) native_paths: Vec<String>,
    pub(crate) native_command: Option<String>,
    pub(crate) trash_dir: Option<String>,
}

#[derive(Serialize)]
struct TrashManifest<'a> {
    recall_session_id: &'a str,
    source: &'a str,
    source_id: &'a str,
    deleted_at_ms: i64,
    original_paths: Vec<String>,
    native_delete_command: Option<String>,
}

pub(crate) fn plan(session: &Session, mode: DeleteMode) -> Result<DeletePlan> {
    if mode == DeleteMode::IndexOnly || session.is_import {
        return Ok(DeletePlan {
            mode: DeleteMode::IndexOnly,
            native_roots: Vec::new(),
            native_command: None,
            suppress_reindex: false,
        });
    }

    let native_command = adapters::delete_command_for(&session.source, &session.source_id);
    let native_roots = native_roots_for_session(session)?;
    if native_command.is_none() && native_roots.is_empty() {
        anyhow::bail!(
            "native deletion is not supported for source {}; use --index-only explicitly to remove only the Recall index",
            session.source
        );
    }

    if mode == DeleteMode::Trash
        && native_command.is_some()
        && native_roots.is_empty()
        && session.source != "opencode"
    {
        anyhow::bail!(
            "source {} has a native delete command but no safe backup path; use Ctrl+D/permanent delete or --index-only explicitly",
            session.source
        );
    }

    Ok(DeletePlan { mode, native_roots, native_command, suppress_reindex: false })
}

pub(crate) fn execute(
    store: &Store,
    session: &Session,
    plan: &DeletePlan,
    dry_run: bool,
) -> Result<DeleteResult> {
    let native_paths = plan
        .native_roots
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let native_command = plan.native_command.as_ref().map(ResumeCommand::display);

    if dry_run {
        return Ok(DeleteResult {
            mode: plan.mode.as_str(),
            deleted_from_index: false,
            native_paths,
            native_command,
            trash_dir: None,
        });
    }

    let mut moved_trash = None;
    let mut trash_dir = None;
    let mut native_deleted_irreversibly = false;
    let opencode_already_missing = session.source == "opencode"
        && crate::adapters::opencode::native_session_exists(&session.source_id)? == Some(false);

    match plan.mode {
        DeleteMode::Trash => {
            if let Some(command) = &plan.native_command {
                if !opencode_already_missing {
                    let backup_dir =
                        backup_before_native_command(session, &plan.native_roots, command)?;
                    if let Err(error) = run_native_delete_command(command) {
                        let native_now_missing = session.source == "opencode"
                            && crate::adapters::opencode::native_session_exists(
                                &session.source_id,
                            )? == Some(false);
                        if !native_now_missing {
                            let _ = fs::remove_dir_all(&backup_dir);
                            return Err(error);
                        }
                    }
                    native_deleted_irreversibly = true;
                    trash_dir = Some(backup_dir);
                }
            } else {
                let moved = move_to_trash(session, &plan.native_roots, None)?;
                trash_dir = Some(moved.dir.clone());
                moved_trash = Some(moved);
            }
        }
        DeleteMode::Permanent => {
            if let Some(command) = &plan.native_command {
                if !opencode_already_missing && let Err(error) = run_native_delete_command(command)
                {
                    let native_now_missing = session.source == "opencode"
                        && crate::adapters::opencode::native_session_exists(&session.source_id)?
                            == Some(false);
                    if !native_now_missing {
                        return Err(error);
                    }
                }
            } else {
                for root in &plan.native_roots {
                    remove_path(root).with_context(|| {
                        format!("failed to permanently delete {}", root.display())
                    })?;
                }
            }
            native_deleted_irreversibly = true;
        }
        DeleteMode::IndexOnly => {}
    }

    if plan.mode != DeleteMode::IndexOnly {
        for root in &plan.native_roots {
            if root.exists() {
                anyhow::bail!(
                    "native deletion reported success but session path still exists: {}",
                    root.display()
                );
            }
        }
        if session.source == "opencode"
            && crate::adapters::opencode::native_session_exists(&session.source_id)? == Some(true)
        {
            anyhow::bail!(
                "OpenCode delete command reported success but session {} still exists in the native database",
                session.source_id
            );
        }
    }

    let delete_index_result = if plan.suppress_reindex {
        store.delete_session_data_with_tombstone(
            &session.source,
            &session.source_id,
            plan.mode.as_str(),
        )
    } else {
        store.delete_session_data(&session.source, &session.source_id)
    };
    if let Err(error) = delete_index_result {
        if let Some(moved) = &moved_trash
            && let Err(rollback_error) = rollback_trash_move(moved)
        {
            return Err(anyhow::anyhow!(
                "failed to delete Recall index: {error}; additionally failed to restore trashed native data: {rollback_error}"
            ));
        }
        if native_deleted_irreversibly {
            let backup_note = trash_dir
                .as_ref()
                .map(|path| format!("; safety backup retained at {}", path.display()))
                .unwrap_or_default();
            return Err(anyhow::anyhow!(
                "native session deletion succeeded, but deleting the Recall index failed: {error}{backup_note}"
            ));
        }
        return Err(error);
    }

    Ok(DeleteResult {
        mode: plan.mode.as_str(),
        deleted_from_index: true,
        native_paths,
        native_command,
        trash_dir: trash_dir.map(|path| path.to_string_lossy().into_owned()),
    })
}

fn native_roots_for_session(session: &Session) -> Result<Vec<PathBuf>> {
    let source_path = session.source_file_path.as_deref().map(PathBuf::from);
    let roots = match session.source.as_str() {
        "codex" => source_path.into_iter().collect(),
        "pi" => source_path
            .and_then(|path| pi_session_file(&path, &session.source_id))
            .into_iter()
            .collect(),
        "oh-my-pi" => source_path
            .map(|path| oh_my_pi_session_roots(&path, &session.source_id))
            .unwrap_or_default(),
        "gemini-cli" => source_path
            .and_then(|path| gemini_session_file(&path, &session.source_id))
            .into_iter()
            .collect(),
        "claude-code" => claude_session_roots(session, source_path.as_deref())?,
        "grok" => source_path
            .and_then(|path| grok_session_root(&path, &session.source_id))
            .into_iter()
            .collect(),
        "copilot-cli" => {
            source_path.and_then(|path| copilot_session_root(&path)).into_iter().collect()
        }
        "cline" => source_path
            .and_then(|path| cline_session_root(&path, &session.source_id))
            .into_iter()
            .collect(),
        "deepseek-harness" => source_path
            .and_then(|path| deepseek_session_root(&path, &session.source_id))
            .into_iter()
            .collect(),
        "kimi-code" => source_path
            .and_then(|path| kimi_session_root(&path, &session.source_id))
            .into_iter()
            .collect(),
        "antigravity-cli" => source_path
            .map(|path| antigravity_session_roots(&path, &session.source_id))
            .unwrap_or_default(),
        // OpenCode has an official deletion command and an export command, so
        // it does not need direct access to the shared SQLite database here.
        "opencode" => Vec::new(),
        // Cursor and Kiro are shared-database sources without a known stable
        // native delete CLI contract. Do not guess writes into their databases.
        "cursor" | "kiro-cli" => Vec::new(),
        _ => Vec::new(),
    };

    normalize_roots(roots)
}

fn pi_session_file(path: &Path, source_id: &str) -> Option<PathBuf> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") || !path.is_file() {
        return None;
    }

    let stem = path.file_stem()?.to_str()?;
    let filename_id = stem
        .rsplit_once('_')
        .map(|(_, tail)| tail)
        .filter(|tail| uuid::Uuid::try_parse(tail).is_ok())
        .unwrap_or(stem);

    let file = fs::File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(256) {
        let Ok(line) = line else { continue };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(|value| value.as_str()) != Some("session") {
            continue;
        }
        return (value.get("id").and_then(|value| value.as_str()) == Some(source_id))
            .then(|| path.to_path_buf());
    }

    (filename_id == source_id).then(|| path.to_path_buf())
}

fn oh_my_pi_session_roots(path: &Path, source_id: &str) -> Vec<PathBuf> {
    let Some(file) = pi_session_file(path, source_id) else {
        return Vec::new();
    };
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let omp_root = home.join(".omp");
    let env_root = std::env::var_os("PI_CODING_AGENT_DIR").map(PathBuf::from).filter(|root| {
        let legacy_pi = home.join(".pi").join("agent");
        fs::canonicalize(root).unwrap_or_else(|_| root.clone())
            != fs::canonicalize(&legacy_pi).unwrap_or(legacy_pi)
    });
    let allowed = canonical_path_is_within(&file, &omp_root)
        || env_root.as_deref().is_some_and(|root| canonical_path_is_within(&file, root));
    if !allowed {
        return Vec::new();
    }

    let mut roots = vec![file.clone()];
    let artifact_dir = file.with_extension("");
    if artifact_dir.is_dir()
        && (canonical_path_is_within(&artifact_dir, &omp_root)
            || env_root
                .as_deref()
                .is_some_and(|root| canonical_path_is_within(&artifact_dir, root)))
    {
        roots.push(artifact_dir);
    }
    roots
}

fn gemini_session_file(path: &Path, source_id: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    gemini_session_file_under(path, source_id, &home.join(".gemini").join("tmp"))
}

fn gemini_session_file_under(path: &Path, source_id: &str, gemini_tmp: &Path) -> Option<PathBuf> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("json") || !path.is_file() {
        return None;
    }
    if path.parent()?.file_name()?.to_str()? != "chats" {
        return None;
    }
    if !canonical_path_is_within(path, gemini_tmp) {
        return None;
    }

    let file = fs::File::open(path).ok()?;
    let value: serde_json::Value = serde_json::from_reader(BufReader::new(file)).ok()?;
    let indexed_id = value
        .get("sessionId")
        .and_then(|value| value.as_str())
        .or_else(|| path.file_stem().and_then(|stem| stem.to_str()))?;
    (indexed_id == source_id).then(|| path.to_path_buf())
}

fn canonical_path_is_within(path: &Path, base: &Path) -> bool {
    let Ok(path) = fs::canonicalize(path) else {
        return false;
    };
    let Ok(base) = fs::canonicalize(base) else {
        return false;
    };
    path != base && path.starts_with(base)
}

fn file_parent_with_name(path: &Path, expected_names: &[&str]) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    if !expected_names.contains(&file_name) {
        return None;
    }
    let parent = path.parent()?;
    parent.parent()?;
    Some(parent.to_path_buf())
}

fn grok_session_root(path: &Path, source_id: &str) -> Option<PathBuf> {
    let root = file_parent_with_name(path, &["updates.jsonl"])?;
    (root.file_name()?.to_str()? == source_id).then_some(root)
}

fn copilot_session_root(path: &Path) -> Option<PathBuf> {
    let root = file_parent_with_name(path, &["events.jsonl"])?;
    let parent_name = root.parent()?.file_name()?.to_str()?;
    (parent_name == "session-state").then_some(root)
}

fn cline_session_root(path: &Path, source_id: &str) -> Option<PathBuf> {
    let root = file_parent_with_name(path, &["ui_messages.json"])?;
    (root.file_name()?.to_str()? == source_id).then_some(root)
}

fn deepseek_session_root(path: &Path, source_id: &str) -> Option<PathBuf> {
    let root = file_parent_with_name(path, &["session.jsonl", "session.jsonl.zstd"])?;
    let encoded = root.file_name()?.to_str()?;
    (crate::adapters::deepseek_harness::decode_dsh_session_id(encoded)? == source_id)
        .then_some(root)
}

fn kimi_session_root(path: &Path, source_id: &str) -> Option<PathBuf> {
    if path.file_name()?.to_str()? != "wire.jsonl" {
        return None;
    }
    let main_dir = path.parent()?;
    if main_dir.file_name()?.to_str()? != "main" {
        return None;
    }
    let agents_dir = main_dir.parent()?;
    if agents_dir.file_name()?.to_str()? != "agents" {
        return None;
    }
    let root = agents_dir.parent()?;
    if !root.file_name()?.to_str()?.starts_with("session_") {
        return None;
    }
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("state.json")).ok()?).ok()?;
    let indexed_id = state
        .get("id")
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| root.file_name().and_then(|name| name.to_str()).unwrap_or(""));
    (indexed_id == source_id).then(|| root.to_path_buf())
}

fn antigravity_session_roots(path: &Path, source_id: &str) -> Vec<PathBuf> {
    if path.file_name().and_then(|name| name.to_str()) != Some("transcript.jsonl") {
        return Vec::new();
    }
    let Some(logs_dir) = path.parent() else {
        return Vec::new();
    };
    if logs_dir.file_name().and_then(|name| name.to_str()) != Some("logs") {
        return Vec::new();
    }
    let Some(generated_dir) = logs_dir.parent() else {
        return Vec::new();
    };
    if generated_dir.file_name().and_then(|name| name.to_str()) != Some(".system_generated") {
        return Vec::new();
    }
    let Some(root) = generated_dir.parent() else {
        return Vec::new();
    };
    if root.file_name().and_then(|name| name.to_str()) != Some(source_id) {
        return Vec::new();
    }
    let Some(brain_dir) = root.parent() else {
        return Vec::new();
    };
    if brain_dir.file_name().and_then(|name| name.to_str()) != Some("brain") {
        return Vec::new();
    }
    let Some(agy_root) = brain_dir.parent() else {
        return Vec::new();
    };
    if agy_root.file_name().and_then(|name| name.to_str()) != Some("antigravity-cli") {
        return Vec::new();
    }

    let mut roots = vec![root.to_path_buf()];
    let conversations = agy_root.join("conversations");
    for extension in ["db", "pb"] {
        let file = conversations.join(format!("{source_id}.{extension}"));
        if file.is_file() {
            roots.push(file);
        }
    }
    let conversation_dir = conversations.join(source_id);
    if conversation_dir.is_dir() {
        roots.push(conversation_dir);
    }
    roots
}

fn claude_session_roots(session: &Session, indexed_path: Option<&Path>) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    let Some(home) = dirs::home_dir() else {
        return Ok(roots);
    };
    let claude_dir = home.join(".claude");
    if let Some(path) = indexed_path
        && claude_indexed_path_is_allowed(path, &session.source_id, &claude_dir)
    {
        roots.push(path.to_path_buf());
    }
    for base in [claude_dir.join("projects"), claude_dir.join("transcripts")] {
        if !base.exists() {
            continue;
        }
        for entry in WalkDir::new(base).into_iter().filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            if path.file_stem().and_then(|stem| stem.to_str()) == Some(session.source_id.as_str()) {
                roots.push(path.to_path_buf());
            }
        }
    }

    let live_meta = claude_dir.join("sessions").join(format!("{}.json", session.source_id));
    if live_meta.is_file() {
        roots.push(live_meta);
    }

    let jobs_dir = claude_dir.join("jobs");
    if let Ok(entries) = fs::read_dir(&jobs_dir) {
        for entry in entries.flatten() {
            let job_dir = entry.path();
            let state_path = job_dir.join("state.json");
            let Ok(state) = fs::read_to_string(state_path) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&state) else {
                continue;
            };
            if value.get("sessionId").and_then(|value| value.as_str())
                == Some(session.source_id.as_str())
                && job_dir.is_dir()
            {
                roots.push(job_dir);
            }
        }
    }

    normalize_roots(roots)
}

fn claude_indexed_path_is_allowed(path: &Path, source_id: &str, claude_dir: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl")
        || path.file_stem().and_then(|stem| stem.to_str()) != Some(source_id)
    {
        return false;
    }

    [claude_dir.join("projects"), claude_dir.join("transcripts")]
        .iter()
        .any(|base| canonical_path_is_within(path, base))
}

fn normalize_roots(roots: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for root in roots {
        let metadata = match fs::symlink_metadata(&root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            anyhow::bail!("refusing to delete symlinked session path: {}", root.display());
        }
        if metadata.is_dir() && root.parent().is_none() {
            anyhow::bail!("refusing to delete a filesystem root: {}", root.display());
        }
        let key = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
        if seen.insert(key) {
            unique.push(root);
        }
    }

    unique.sort_by_key(|path| path.components().count());
    let mut compact = Vec::new();
    for root in unique {
        if compact.iter().any(|parent: &PathBuf| root.starts_with(parent)) {
            continue;
        }
        compact.push(root);
    }
    Ok(compact)
}

fn run_native_delete_command(command: &ResumeCommand) -> Result<()> {
    let output = Command::new(&command.program)
        .args(&command.args)
        .output()
        .with_context(|| format!("failed to start native delete command: {}", command.display()))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!(
            "native delete command failed with status {}: {}{}",
            output.status,
            command.display(),
            if detail.is_empty() { String::new() } else { format!(": {detail}") }
        );
    }
    Ok(())
}

fn backup_before_native_command(
    session: &Session,
    roots: &[PathBuf],
    command: &ResumeCommand,
) -> Result<PathBuf> {
    let deleted_at_ms = Utc::now().timestamp_millis();
    let trash_dir = new_trash_dir(session, deleted_at_ms)?;
    if let Err(error) =
        write_manifest(session, roots, &trash_dir, deleted_at_ms, Some(command.display()))
    {
        let _ = fs::remove_dir_all(&trash_dir);
        return Err(error);
    }

    let backup_result = if session.source == "opencode" {
        backup_opencode_export(session, &trash_dir)
    } else {
        backup_paths(roots, &trash_dir)
    };
    if let Err(error) = backup_result {
        let _ = fs::remove_dir_all(&trash_dir);
        return Err(error);
    }
    Ok(trash_dir)
}

fn backup_opencode_export(session: &Session, trash_dir: &Path) -> Result<()> {
    let command = crate::adapters::opencode::opencode_maintenance_command(vec![
        "export".to_string(),
        session.source_id.clone(),
    ]);
    let export_path = trash_dir.join("session-export.json");
    let export_file = fs::File::create(&export_path)
        .with_context(|| format!("failed to create {}", export_path.display()))?;
    let output = Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::null())
        // OpenCode session exports can be large. Writing directly to the
        // backup file avoids its stdout pipe path, which has had truncation
        // issues upstream and can leave the TUI waiting indefinitely.
        .stdout(Stdio::from(export_file))
        .stderr(Stdio::piped())
        .output()
        .context("failed to start `opencode export` for safety backup")?;
    if !output.status.success() {
        let _ = fs::remove_file(&export_path);
        anyhow::bail!(
            "`opencode export` failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if fs::metadata(&export_path).map(|metadata| metadata.len()).unwrap_or_default() == 0 {
        let _ = fs::remove_file(&export_path);
        anyhow::bail!("`opencode export` returned an empty safety backup");
    }
    let export = fs::File::open(&export_path)
        .with_context(|| format!("failed to reopen {}", export_path.display()))?;
    if let Err(error) = serde_json::from_reader::<_, serde_json::Value>(export) {
        let _ = fs::remove_file(&export_path);
        return Err(error).context("`opencode export` returned an invalid JSON safety backup");
    }
    Ok(())
}

fn backup_paths(roots: &[PathBuf], trash_dir: &Path) -> Result<()> {
    if roots.is_empty() {
        anyhow::bail!("no native session files are available for a safety backup");
    }
    for (index, root) in roots.iter().enumerate() {
        let dest = trash_path_for_root(trash_dir, index, root);
        copy_path(root, &dest).with_context(|| {
            format!("failed to back up {} to {}", root.display(), dest.display())
        })?;
    }
    Ok(())
}

fn move_to_trash(
    session: &Session,
    roots: &[PathBuf],
    native_command: Option<String>,
) -> Result<TrashMove> {
    let deleted_at_ms = Utc::now().timestamp_millis();
    let trash_dir = new_trash_dir(session, deleted_at_ms)?;
    move_to_trash_at(session, roots, &trash_dir, deleted_at_ms, native_command)
}

#[derive(Debug)]
struct TrashMove {
    dir: PathBuf,
    moved: Vec<(PathBuf, PathBuf)>,
}

fn new_trash_dir(session: &Session, deleted_at_ms: i64) -> Result<PathBuf> {
    let data_dir =
        dirs::data_dir().ok_or_else(|| anyhow::anyhow!("cannot determine data directory"))?;
    let trash_dir = data_dir.join("recall").join("trash").join(format!(
        "{}-{}-{}",
        deleted_at_ms,
        sanitize_component(&session.source),
        sanitize_component(&session.source_id)
    ));
    fs::create_dir_all(&trash_dir)?;
    Ok(trash_dir)
}

fn write_manifest(
    session: &Session,
    roots: &[PathBuf],
    trash_dir: &Path,
    deleted_at_ms: i64,
    native_delete_command: Option<String>,
) -> Result<()> {
    let manifest = TrashManifest {
        recall_session_id: &session.id,
        source: &session.source,
        source_id: &session.source_id,
        deleted_at_ms,
        original_paths: roots.iter().map(|path| path.to_string_lossy().into_owned()).collect(),
        native_delete_command,
    };
    fs::write(trash_dir.join("manifest.json"), serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}

fn move_to_trash_at(
    session: &Session,
    roots: &[PathBuf],
    trash_dir: &Path,
    deleted_at_ms: i64,
    native_command: Option<String>,
) -> Result<TrashMove> {
    fs::create_dir_all(trash_dir)?;
    write_manifest(session, roots, trash_dir, deleted_at_ms, native_command)?;

    let mut moved = Vec::new();
    for (index, root) in roots.iter().enumerate() {
        let dest = trash_path_for_root(trash_dir, index, root);
        if let Err(error) = move_path(root, &dest) {
            let rollback = TrashMove { dir: trash_dir.to_path_buf(), moved };
            let _ = rollback_trash_move(&rollback);
            return Err(error).with_context(|| {
                format!("failed to move {} to {}", root.display(), dest.display())
            });
        }
        moved.push((root.clone(), dest));
    }
    Ok(TrashMove { dir: trash_dir.to_path_buf(), moved })
}

fn trash_path_for_root(trash_dir: &Path, index: usize, root: &Path) -> PathBuf {
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "session-data".to_string());
    trash_dir.join(format!("{index}-{name}"))
}

fn rollback_trash_move(trash_move: &TrashMove) -> Result<()> {
    for (original, trashed) in trash_move.moved.iter().rev() {
        if trashed.exists() {
            move_path(trashed, original).with_context(|| {
                format!("failed to restore {} to {}", trashed.display(), original.display())
            })?;
        }
    }
    if trash_move.dir.exists() {
        fs::remove_dir_all(&trash_move.dir)?;
    }
    Ok(())
}

fn move_path(source: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(source, dest) {
        Ok(()) => return Ok(()),
        Err(error) if is_cross_device_error(&error) => {}
        Err(error) => return Err(error.into()),
    }

    if let Err(error) = copy_path(source, dest) {
        let _ = remove_path(dest);
        return Err(error);
    }
    if let Err(error) = remove_path(source) {
        let _ = remove_path(dest);
        return Err(error);
    }
    Ok(())
}

fn copy_path(source: &Path, dest: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        anyhow::bail!("refusing to copy symlinked session path: {}", source.display());
    }
    if metadata.is_file() {
        fs::copy(source, dest)?;
        return Ok(());
    }
    if metadata.is_dir() {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_path(&entry.path(), &dest.join(entry.file_name()))?;
        }
        return Ok(());
    }
    anyhow::bail!("unsupported session path type: {}", source.display())
}

fn remove_path(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        anyhow::bail!("refusing to delete symlinked session path: {}", path.display());
    }
    if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn is_cross_device_error(error: &io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(18)
    }
    #[cfg(windows)]
    {
        error.raw_os_error() == Some(17)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = error;
        false
    }
}

fn sanitize_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() { "session".to_string() } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(source: &str, source_id: &str, source_file_path: Option<String>) -> Session {
        Session {
            id: "recall-id".to_string(),
            source: source.to_string(),
            source_id: source_id.to_string(),
            title: "title".to_string(),
            directory: None,
            repo_remote: None,
            repo_slug: None,
            repo_name: None,
            started_at: 0,
            updated_at: None,
            message_count: 0,
            entrypoint: None,
            custom_title: None,
            summary: None,
            duration_minutes: None,
            source_file_path,
            is_import: false,
        }
    }

    #[test]
    fn file_backed_sources_plan_exact_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        fs::write(
            &path,
            r#"{"type":"session","id":"s1"}
"#,
        )
        .unwrap();
        let session = session("pi", "s1", Some(path.to_string_lossy().into_owned()));

        let plan = plan(&session, DeleteMode::Trash).unwrap();

        assert_eq!(plan.native_roots, vec![path]);
        assert!(plan.native_command.is_none());
    }

    #[test]
    fn pi_rejects_mismatched_session_header_even_when_filename_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("expected.jsonl");
        fs::write(&path, "{\"type\":\"session\",\"id\":\"other\"}\n").unwrap();

        assert!(pi_session_file(&path, "expected").is_none());
        assert!(path.exists());
    }

    #[test]
    fn pi_accepts_matching_session_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unrelated-name.jsonl");
        fs::write(&path, "{\"type\":\"session\",\"id\":\"pi-session\"}\n").unwrap();

        assert_eq!(pi_session_file(&path, "pi-session"), Some(path));
    }

    #[test]
    fn gemini_session_path_requires_chats_tree_and_matching_id() {
        let dir = tempfile::tempdir().unwrap();
        let gemini_tmp = dir.path().join("tmp");
        let chats = gemini_tmp.join("project-hash").join("chats");
        fs::create_dir_all(&chats).unwrap();
        let path = chats.join("session.json");
        fs::write(&path, r#"{"sessionId":"gemini-session","messages":[]}"#).unwrap();

        assert_eq!(
            gemini_session_file_under(&path, "gemini-session", &gemini_tmp),
            Some(path.clone())
        );
        assert!(gemini_session_file_under(&path, "other", &gemini_tmp).is_none());

        let outside = dir.path().join("outside").join("chats");
        fs::create_dir_all(&outside).unwrap();
        let outside_path = outside.join("session.json");
        fs::write(&outside_path, r#"{"sessionId":"gemini-session"}"#).unwrap();
        assert!(gemini_session_file_under(&outside_path, "gemini-session", &gemini_tmp).is_none());
    }

    #[test]
    fn claude_indexed_path_must_be_inside_known_claude_tree() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        let projects = claude_dir.join("projects").join("project");
        fs::create_dir_all(&projects).unwrap();
        let allowed = projects.join("session-id.jsonl");
        fs::write(&allowed, "{}\n").unwrap();

        assert!(claude_indexed_path_is_allowed(&allowed, "session-id", &claude_dir));
        assert!(!claude_indexed_path_is_allowed(&allowed, "other-id", &claude_dir));

        let outside = dir.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let outside_path = outside.join("session-id.jsonl");
        fs::write(&outside_path, "{}\n").unwrap();
        assert!(!claude_indexed_path_is_allowed(&outside_path, "session-id", &claude_dir));
    }

    #[test]
    fn codex_uses_native_delete_command_and_keeps_file_for_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        fs::write(&path, "data").unwrap();
        let session = session(
            "codex",
            "11111111-1111-1111-1111-111111111111",
            Some(path.to_string_lossy().into_owned()),
        );

        let plan = plan(&session, DeleteMode::Trash).unwrap();
        let command = plan.native_command.unwrap();

        assert_eq!(plan.native_roots, vec![path]);
        #[cfg(target_os = "windows")]
        {
            assert_eq!(command.program, "cmd.exe");
            assert_eq!(
                command.args,
                vec![
                    "/D",
                    "/C",
                    "codex",
                    "delete",
                    "--force",
                    "11111111-1111-1111-1111-111111111111"
                ]
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(command.program, "codex");
            assert_eq!(
                command.args,
                vec!["delete", "--force", "11111111-1111-1111-1111-111111111111"]
            );
        }
    }

    #[test]
    fn opencode_uses_native_delete_command_without_database_writes() {
        let session = session("opencode", "ses_123", None);

        let plan = plan(&session, DeleteMode::Trash).unwrap();
        let command = plan.native_command.unwrap();

        assert!(plan.native_roots.is_empty());
        #[cfg(target_os = "windows")]
        {
            assert_eq!(command.program, "cmd.exe");
            assert_eq!(
                command.args,
                vec!["/D", "/C", "opencode", "--pure", "session", "delete", "ses_123"]
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(command.program, "opencode");
            assert_eq!(command.args, vec!["--pure", "session", "delete", "ses_123"]);
        }
    }

    #[test]
    fn directory_backed_sources_plan_session_directory() {
        let dir = tempfile::tempdir().unwrap();
        let session_dir = dir.path().join("session-state").join("session-id");
        fs::create_dir_all(&session_dir).unwrap();
        let path = session_dir.join("events.jsonl");
        fs::write(&path, "data").unwrap();
        let session = session("copilot-cli", "s1", Some(path.to_string_lossy().into_owned()));

        let plan = plan(&session, DeleteMode::Trash).unwrap();

        assert_eq!(plan.native_roots, vec![session_dir]);
    }

    #[test]
    fn malformed_directory_source_path_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let suspicious_dir = dir.path().join("not-session-state").join("session-id");
        fs::create_dir_all(&suspicious_dir).unwrap();
        let path = suspicious_dir.join("events.jsonl");
        fs::write(&path, "data").unwrap();
        let session = session("copilot-cli", "s1", Some(path.to_string_lossy().into_owned()));

        let error = plan(&session, DeleteMode::Trash).unwrap_err();
        assert!(error.to_string().contains("native deletion is not supported"));
        assert!(path.exists());
    }

    #[test]
    fn successful_native_command_cannot_hide_a_still_existing_session_path() {
        crate::db::schema::register_sqlite_vec();
        let store = Store::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("native-session.jsonl");
        fs::write(&path, "still here").unwrap();
        let session = session("codex", "11111111-1111-1111-1111-111111111111", None);
        store.insert_session(&session).unwrap();

        #[cfg(target_os = "windows")]
        let command = ResumeCommand {
            program: "cmd.exe".to_string(),
            args: vec!["/D".to_string(), "/C".to_string(), "exit".to_string(), "0".to_string()],
        };
        #[cfg(not(target_os = "windows"))]
        let command = ResumeCommand {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 0".to_string()],
        };
        let delete_plan = DeletePlan {
            mode: DeleteMode::Permanent,
            native_roots: vec![path.clone()],
            native_command: Some(command),
            suppress_reindex: false,
        };

        let error = execute(&store, &session, &delete_plan, false).unwrap_err();
        assert!(error.to_string().contains("session path still exists"));
        assert!(path.exists());
        assert!(store.get_session_by_id(&session.id).unwrap().is_some());
    }

    #[test]
    fn unsupported_shared_database_sources_require_explicit_index_only() {
        let session = session("cursor", "s1", None);

        let trash = plan(&session, DeleteMode::Trash).unwrap_err();
        assert!(trash.to_string().contains("use --index-only explicitly"));
        let permanent = plan(&session, DeleteMode::Permanent).unwrap_err();
        assert!(permanent.to_string().contains("use --index-only explicitly"));
    }

    #[test]
    fn unsupported_native_delete_does_not_touch_index() {
        crate::db::schema::register_sqlite_vec();
        let store = Store::open_in_memory().unwrap();
        let session = session("cursor", "cursor-native-id", None);
        store
            .conn
            .execute(
                "INSERT INTO sessions (id, source, source_id, title, started_at)
                 VALUES (?1, ?2, ?3, ?4, 0)",
                rusqlite::params![session.id, session.source, session.source_id, session.title],
            )
            .unwrap();

        assert!(plan(&session, DeleteMode::Trash).is_err());
        assert!(store.get_session_by_id(&session.id).unwrap().is_some());
        assert!(!store.session_is_tombstoned(&session.source, &session.source_id).unwrap());
    }

    #[test]
    fn antigravity_delete_includes_brain_and_canonical_conversation_files() {
        let dir = tempfile::tempdir().unwrap();
        let agy = dir.path().join("antigravity-cli");
        let id = "11111111-1111-1111-1111-111111111111";
        let brain = agy.join("brain").join(id);
        let logs = brain.join(".system_generated").join("logs");
        let conversations = agy.join("conversations");
        fs::create_dir_all(&logs).unwrap();
        fs::create_dir_all(&conversations).unwrap();
        let transcript = logs.join("transcript.jsonl");
        let db = conversations.join(format!("{id}.db"));
        let pb = conversations.join(format!("{id}.pb"));
        fs::write(&transcript, "{}\n").unwrap();
        fs::write(&db, "db").unwrap();
        fs::write(&pb, "pb").unwrap();

        let mut roots = antigravity_session_roots(&transcript, id);
        roots.sort();
        let mut expected = vec![brain, db, pb];
        expected.sort();
        assert_eq!(roots, expected);
    }

    #[test]
    fn explicit_index_only_does_not_write_tombstone() {
        crate::db::schema::register_sqlite_vec();
        let store = Store::open_in_memory().unwrap();
        let session = session("cursor", "cursor-native-id", None);
        store
            .conn
            .execute(
                "INSERT INTO sessions (id, source, source_id, title, started_at)
                 VALUES (?1, ?2, ?3, ?4, 0)",
                rusqlite::params![session.id, session.source, session.source_id, session.title],
            )
            .unwrap();

        let plan = plan(&session, DeleteMode::IndexOnly).unwrap();
        execute(&store, &session, &plan, false).unwrap();

        assert!(!store.session_is_tombstoned(&session.source, &session.source_id).unwrap());
    }

    #[test]
    fn trash_move_writes_manifest_and_moves_data() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.jsonl");
        fs::write(&source, "payload").unwrap();
        let session = session("pi", "native-id", Some(source.to_string_lossy().into_owned()));
        let trash = dir.path().join("trash-entry");

        move_to_trash_at(&session, std::slice::from_ref(&source), &trash, 123, None).unwrap();

        assert!(!source.exists());
        assert_eq!(fs::read_to_string(trash.join("0-source.jsonl")).unwrap(), "payload");
        let manifest = fs::read_to_string(trash.join("manifest.json")).unwrap();
        assert!(manifest.contains("native-id"));
        assert!(manifest.contains("123"));
    }
}
