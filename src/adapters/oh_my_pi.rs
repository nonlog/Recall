use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde_json::Value;
use tracing::debug;
use walkdir::WalkDir;

use crate::adapters::file_scan::{self, FileScanEntry};
use crate::adapters::pi::parse_pi_session;
use crate::adapters::{
    RawSession, ResumeCommand, SourceAdapter, SyncScanResult, SyncScanStats, first_timestamp,
};
use crate::db::store::Store;
use crate::types::{ParentLink, ParentRelation, ThreadRole};

pub(crate) struct OhMyPiAdapter;

const USAGE_PARSER_VERSION: u32 = 1;
const METADATA_PARSER_VERSION: u32 = 1;

impl SourceAdapter for OhMyPiAdapter {
    fn id(&self) -> &str {
        "oh-my-pi"
    }

    fn label(&self) -> &str {
        "OMP"
    }

    fn resume_command(&self, source_id: &str) -> Option<ResumeCommand> {
        Some(ResumeCommand {
            program: "omp".to_string(),
            args: vec!["--resume".to_string(), source_id.to_string()],
        })
    }

    fn usage_parser_version(&self) -> Option<u32> {
        Some(USAGE_PARSER_VERSION)
    }

    fn scan(&self) -> anyhow::Result<Vec<RawSession>> {
        let dirs = resolve_omp_session_dirs()?;
        let mut sessions = Vec::new();
        for entry in collect_omp_entries(&dirs) {
            let Some(mtime_ms) = file_scan::stat_mtime_ms(&entry.stat_target) else {
                continue;
            };
            if let Some(session) = parse_omp_session_file(entry, mtime_ms)? {
                sessions.push(session);
            }
        }
        Ok(sessions)
    }

    fn scan_for_sync(
        &self,
        store: &Store,
        since_ts: Option<i64>,
        include_events: bool,
    ) -> anyhow::Result<Option<SyncScanResult>> {
        let dirs = resolve_omp_session_dirs()?;
        if dirs.is_empty() {
            return Ok(Some(SyncScanResult { sessions: vec![], stats: SyncScanStats::default() }));
        }
        let entries = collect_omp_entries(&dirs);
        Ok(Some(file_scan::run_file_scan_with_options(
            store,
            "oh-my-pi",
            since_ts,
            file_scan::FileScanOptions {
                usage_parser_version: Some(USAGE_PARSER_VERSION),
                event_parser_version: None,
                metadata_parser_version: include_events.then_some(METADATA_PARSER_VERSION),
            },
            entries,
            parse_omp_session_file,
        )?))
    }
}

fn resolve_omp_session_dirs() -> anyhow::Result<Vec<PathBuf>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    let config_root = home.join(".omp");
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();

    push_existing_unique_dir(&mut dirs, &mut seen, config_root.join("agent").join("sessions"));

    let profiles = config_root.join("profiles");
    if let Ok(entries) = fs::read_dir(profiles) {
        for entry in entries.flatten() {
            push_existing_unique_dir(
                &mut dirs,
                &mut seen,
                entry.path().join("agent").join("sessions"),
            );
        }
    }

    // OMP documents PI_CODING_AGENT_DIR as an override. Only add it when it
    // is clearly distinct from the legacy ~/.pi agent tree, otherwise Recall
    // would double-index Pi sessions as OMP.
    if let Some(agent_dir) = std::env::var_os("PI_CODING_AGENT_DIR") {
        let agent_dir = PathBuf::from(agent_dir);
        let legacy_pi = home.join(".pi").join("agent");
        let resolved = fs::canonicalize(&agent_dir).unwrap_or_else(|_| agent_dir.clone());
        let legacy = fs::canonicalize(&legacy_pi).unwrap_or(legacy_pi);
        if resolved != legacy {
            push_existing_unique_dir(&mut dirs, &mut seen, agent_dir.join("sessions"));
        }
    }

    if dirs.is_empty() {
        debug!("Oh My Pi session directory not found, skipping OMP");
    }
    Ok(dirs)
}

fn push_existing_unique_dir(dirs: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, dir: PathBuf) {
    if !dir.is_dir() {
        return;
    }
    let key = fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
    if seen.insert(key) {
        dirs.push(dir);
    }
}

fn collect_omp_entries(session_dirs: &[PathBuf]) -> Vec<FileScanEntry> {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for dir in session_dirs {
        for entry in WalkDir::new(dir).into_iter().filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            // Subagent transcripts live in the sibling artifact directory
            // `<session-file-without-.jsonl>/...`; only index top-level session
            // JSONLs here, not those nested child transcripts.
            if is_omp_artifact_session(path, dir) {
                continue;
            }
            let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            if !seen.insert(canonical) {
                continue;
            }
            let Some(session_id) = omp_session_id(path) else {
                continue;
            };
            entries.push(FileScanEntry {
                session_id,
                stat_target: path.to_path_buf(),
                directory: None,
            });
        }
    }
    entries
}

fn is_omp_artifact_session(path: &Path, sessions_root: &Path) -> bool {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == sessions_root || !dir.starts_with(sessions_root) {
            break;
        }
        if dir.with_extension("jsonl").is_file() {
            return true;
        }
        current = dir.parent();
    }
    false
}

fn omp_session_id(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(4) {
        let Ok(line) = line else { continue };
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if value.get("type").and_then(|value| value.as_str()) == Some("session") {
            return value
                .get("id")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
        }
    }
    None
}

fn parse_omp_session_file(
    entry: FileScanEntry,
    mtime_ms: i64,
) -> anyhow::Result<Option<RawSession>> {
    let source_file_path = entry.stat_target.to_str().map(str::to_string);
    let parsed = match parse_pi_session(&entry.stat_target, mtime_ms) {
        Ok(parsed) => parsed,
        Err(error) => {
            debug!("failed to parse OMP session {}: {error}", entry.stat_target.display());
            return Ok(None);
        }
    };
    if parsed.messages.is_empty() && parsed.usage_events.is_empty() {
        return Ok(None);
    }
    let started_at =
        first_timestamp(parsed.started_at, &parsed.messages, &parsed.usage_events, &[])
            .unwrap_or(0);
    let source_id = parsed.session_id.unwrap_or(entry.session_id);
    let parent_links = parsed
        .parent_session
        .as_deref()
        .and_then(omp_parent_id)
        .filter(|parent| parent != &source_id)
        .map(|parent| {
            vec![ParentLink {
                relation: ParentRelation::Fork,
                source: "oh-my-pi".to_string(),
                source_id: parent,
            }]
        })
        .unwrap_or_default();

    Ok(Some(RawSession {
        source_id,
        directory: parsed.cwd,
        started_at,
        updated_at: Some(mtime_ms),
        entrypoint: None,
        messages: parsed.messages,
        usage_events: parsed.usage_events,
        usage_parser_version: Some(USAGE_PARSER_VERSION),
        events: Vec::new(),
        event_parser_version: None,
        source_file_path,
        custom_title: parsed.title,
        summary: None,
        duration_minutes: None,
        thread_role: Some(ThreadRole::Primary),
        parent_links,
        metadata_parser_version: Some(METADATA_PARSER_VERSION),
    }))
}

fn omp_parent_id(parent: &str) -> Option<String> {
    if uuid::Uuid::try_parse(parent).is_ok() {
        return Some(parent.to_string());
    }
    let stem = Path::new(parent).file_stem()?.to_str()?;
    let candidate = stem.rsplit_once('_').map(|(_, tail)| tail).unwrap_or(stem);
    uuid::Uuid::try_parse(candidate).ok().map(|_| candidate.to_string())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn omp_parser_prefers_native_fixed_slot_title() {
        let path = std::env::temp_dir().join(format!("recall-omp-{}.jsonl", uuid::Uuid::new_v4()));
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"title","v":1,"title":"Native OMP title","source":"auto","updatedAt":"2026-08-22T08:58:05Z","pad":""}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"session","version":3,"id":"01a028b0-ef48-766d-922e-6627ed104450","timestamp":"2026-08-22T08:57:59Z","cwd":"D:\\work","title":"Older title"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"title_change","id":"abcd1234","parentId":null,"timestamp":"2026-08-22T08:58:06Z","title":"Latest OMP title","source":"user"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"abcd1235","parentId":"abcd1234","timestamp":"2026-08-22T08:58:07Z","message":{{"role":"user","content":[{{"type":"text","text":"raw prompt"}}],"timestamp":1787389081222}}}}"#
        )
        .unwrap();
        drop(file);

        let entry = FileScanEntry {
            session_id: "fallback".to_string(),
            stat_target: path.clone(),
            directory: None,
        };
        let raw = parse_omp_session_file(entry, 1).unwrap().unwrap();
        assert_eq!(raw.source_id, "01a028b0-ef48-766d-922e-6627ed104450");
        assert_eq!(raw.custom_title.as_deref(), Some("Latest OMP title"));
        assert_eq!(raw.messages[0].content, "raw prompt");
        let _ = fs::remove_file(path);
    }
}
