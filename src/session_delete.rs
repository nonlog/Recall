use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use walkdir::WalkDir;

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
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeleteResult {
    pub(crate) mode: &'static str,
    pub(crate) deleted_from_index: bool,
    pub(crate) native_paths: Vec<String>,
    pub(crate) trash_dir: Option<String>,
}

#[derive(Serialize)]
struct TrashManifest<'a> {
    recall_session_id: &'a str,
    source: &'a str,
    source_id: &'a str,
    deleted_at_ms: i64,
    original_paths: Vec<String>,
}

pub(crate) fn plan(session: &Session, mode: DeleteMode) -> Result<DeletePlan> {
    if mode == DeleteMode::IndexOnly || session.is_import {
        return Ok(DeletePlan { mode: DeleteMode::IndexOnly, native_roots: Vec::new() });
    }

    let native_roots = native_roots_for_session(session)?;
    if native_roots.is_empty() {
        anyhow::bail!(
            "native deletion is not supported for source {}; use --index-only to remove only the Recall index entry",
            session.source
        );
    }

    Ok(DeletePlan { mode, native_roots })
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

    if dry_run {
        return Ok(DeleteResult {
            mode: plan.mode.as_str(),
            deleted_from_index: false,
            native_paths,
            trash_dir: None,
        });
    }

    let trash_move = match plan.mode {
        DeleteMode::Trash => Some(move_to_trash(session, &plan.native_roots)?),
        DeleteMode::Permanent => {
            for root in &plan.native_roots {
                remove_path(root)
                    .with_context(|| format!("failed to permanently delete {}", root.display()))?;
            }
            None
        }
        DeleteMode::IndexOnly => None,
    };

    if let Err(error) = store.delete_session_data(&session.source, &session.source_id) {
        if let Some(trash_move) = &trash_move {
            if let Err(rollback_error) = rollback_trash_move(trash_move) {
                return Err(anyhow::anyhow!(
                    "failed to delete Recall index: {error}; additionally failed to restore trashed native data: {rollback_error}"
                ));
            }
        }
        return Err(error);
    }

    Ok(DeleteResult {
        mode: plan.mode.as_str(),
        deleted_from_index: true,
        native_paths,
        trash_dir: trash_move.map(|moved| moved.dir.to_string_lossy().into_owned()),
    })
}

fn native_roots_for_session(session: &Session) -> Result<Vec<PathBuf>> {
    let source_path = session.source_file_path.as_deref().map(PathBuf::from);
    let roots = match session.source.as_str() {
        "codex" | "pi" | "gemini-cli" => source_path.into_iter().collect(),
        "claude-code" => claude_session_roots(session, source_path.as_deref())?,
        "grok" | "copilot-cli" | "cline" | "deepseek-harness" => {
            source_path.and_then(|path| path.parent().map(Path::to_path_buf)).into_iter().collect()
        }
        "kimi-code" => source_path
            .and_then(|path| path.ancestors().nth(3).map(Path::to_path_buf))
            .into_iter()
            .collect(),
        "antigravity-cli" => source_path
            .and_then(|path| path.ancestors().nth(3).map(Path::to_path_buf))
            .into_iter()
            .collect(),
        // These adapters read shared SQLite state. Native deletion needs a
        // source-specific transaction and is deliberately not guessed here.
        "opencode" | "cursor" | "kiro-cli" => Vec::new(),
        _ => Vec::new(),
    };

    normalize_roots(roots)
}

fn claude_session_roots(session: &Session, indexed_path: Option<&Path>) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    if let Some(path) = indexed_path {
        roots.push(path.to_path_buf());
    }

    let Some(home) = dirs::home_dir() else {
        return normalize_roots(roots);
    };
    let claude_dir = home.join(".claude");
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

    normalize_roots(roots)
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

fn move_to_trash(session: &Session, roots: &[PathBuf]) -> Result<TrashMove> {
    let data_dir =
        dirs::data_dir().ok_or_else(|| anyhow::anyhow!("cannot determine data directory"))?;
    let deleted_at_ms = Utc::now().timestamp_millis();
    let trash_dir = data_dir.join("recall").join("trash").join(format!(
        "{}-{}-{}",
        deleted_at_ms,
        sanitize_component(&session.source),
        sanitize_component(&session.source_id)
    ));
    move_to_trash_at(session, roots, &trash_dir, deleted_at_ms)
}

#[derive(Debug)]
struct TrashMove {
    dir: PathBuf,
    moved: Vec<(PathBuf, PathBuf)>,
}

fn move_to_trash_at(
    session: &Session,
    roots: &[PathBuf],
    trash_dir: &Path,
    deleted_at_ms: i64,
) -> Result<TrashMove> {
    fs::create_dir_all(trash_dir)?;
    let manifest = TrashManifest {
        recall_session_id: &session.id,
        source: &session.source,
        source_id: &session.source_id,
        deleted_at_ms,
        original_paths: roots
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    };
    fs::write(
        trash_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;

    let mut moved = Vec::new();
    for (index, root) in roots.iter().enumerate() {
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "session-data".to_string());
        let dest = trash_dir.join(format!("{index}-{name}"));
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

fn rollback_trash_move(trash_move: &TrashMove) -> Result<()> {
    for (original, trashed) in trash_move.moved.iter().rev() {
        if trashed.exists() {
            move_path(trashed, original).with_context(|| {
                format!(
                    "failed to restore {} to {}",
                    trashed.display(),
                    original.display()
                )
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
        fs::write(&path, "data").unwrap();
        let session = session("codex", "s1", Some(path.to_string_lossy().into_owned()));

        let plan = plan(&session, DeleteMode::Trash).unwrap();

        assert_eq!(plan.native_roots, vec![path]);
    }

    #[test]
    fn directory_backed_sources_plan_session_directory() {
        let dir = tempfile::tempdir().unwrap();
        let session_dir = dir.path().join("session-id");
        fs::create_dir_all(&session_dir).unwrap();
        let path = session_dir.join("events.jsonl");
        fs::write(&path, "data").unwrap();
        let session = session("copilot-cli", "s1", Some(path.to_string_lossy().into_owned()));

        let plan = plan(&session, DeleteMode::Trash).unwrap();

        assert_eq!(plan.native_roots, vec![session_dir]);
    }

    #[test]
    fn shared_database_sources_require_index_only() {
        let session = session("opencode", "s1", None);

        let err = plan(&session, DeleteMode::Trash).unwrap_err();
        assert!(err.to_string().contains("--index-only"));
        let plan = plan(&session, DeleteMode::IndexOnly).unwrap();
        assert!(plan.native_roots.is_empty());
    }

    #[test]
    fn trash_move_writes_manifest_and_moves_data() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.jsonl");
        fs::write(&source, "payload").unwrap();
        let session = session("codex", "native-id", Some(source.to_string_lossy().into_owned()));
        let trash = dir.path().join("trash-entry");

        move_to_trash_at(&session, std::slice::from_ref(&source), &trash, 123).unwrap();

        assert!(!source.exists());
        assert_eq!(fs::read_to_string(trash.join("0-source.jsonl")).unwrap(), "payload");
        let manifest = fs::read_to_string(trash.join("manifest.json")).unwrap();
        assert!(manifest.contains("native-id"));
        assert!(manifest.contains("123"));
    }
}
