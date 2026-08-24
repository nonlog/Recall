from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text()
    if new in text:
        return
    if old not in text:
        raise SystemExit(f"marker not found in {path}: {old[:80]!r}")
    file_path.write_text(text.replace(old, new, 1))


replace_once(
    "src/lib.rs",
    "pub(crate) mod session;\n",
    "pub(crate) mod session;\npub(crate) mod session_delete;\n",
)

handoff_variant = '''    #[command(about = "Handoff one selected session to a new target agent session")]
    Handoff {
'''
delete_variant = '''    #[command(about = "Delete one indexed session")]
    Delete {
        #[arg(long, help = "Recall session id")]
        id: Option<String>,
        #[arg(long, help = "Source id or label")]
        source: Option<String>,
        #[arg(long, help = "Source-native session id")]
        source_id: Option<String>,
        #[arg(
            long,
            conflicts_with = "index_only",
            help = "Permanently delete native session data instead of moving it to Recall trash"
        )]
        permanent: bool,
        #[arg(
            long,
            conflicts_with = "permanent",
            help = "Delete only the Recall index entry and leave native session data untouched"
        )]
        index_only: bool,
        #[arg(long, help = "Show what would be deleted without changing anything")]
        dry_run: bool,
        #[arg(long, value_enum, default_value_t = SessionActionFormat::Text)]
        format: SessionActionFormat,
    },
'''
replace_once("src/session.rs", handoff_variant, delete_variant + handoff_variant)

handoff_match = '''        SessionCommands::Handoff { id, source, source_id, to, print_prompt } => {
            cmd_session_handoff(
'''
delete_match = '''        SessionCommands::Delete {
            id,
            source,
            source_id,
            permanent,
            index_only,
            dry_run,
            format,
        } => cmd_session_delete(
            id.as_deref(),
            source.as_deref(),
            source_id.as_deref(),
            permanent,
            index_only,
            dry_run,
            format,
        ),
'''
replace_once("src/session.rs", handoff_match, delete_match + handoff_match)

delete_fn = r'''fn cmd_session_delete(
    id: Option<&str>,
    source_filter: Option<&str>,
    source_id: Option<&str>,
    permanent: bool,
    index_only: bool,
    dry_run: bool,
    format: SessionActionFormat,
) -> Result<()> {
    let store = Store::open()?;
    let sources = adapters::source_labels();
    let session = resolve_session_ref(&store, &sources, id, source_filter, source_id)?;
    let mode = if index_only {
        crate::session_delete::DeleteMode::IndexOnly
    } else if permanent {
        crate::session_delete::DeleteMode::Permanent
    } else {
        crate::session_delete::DeleteMode::Trash
    };
    let plan = crate::session_delete::plan(&session, mode)?;
    let result = crate::session_delete::execute(&store, &session, &plan, dry_run)?;

    match format {
        SessionActionFormat::Text => {
            if dry_run {
                println!(
                    "Dry run: would delete session {} ({}/{}) using {} mode",
                    session.id, session.source, session.source_id, result.mode
                );
            } else {
                println!(
                    "Deleted session {} ({}/{}) using {} mode",
                    session.id, session.source, session.source_id, result.mode
                );
            }
            for path in &result.native_paths {
                println!("  native: {path}");
            }
            if let Some(trash_dir) = &result.trash_dir {
                println!("  trash: {trash_dir}");
            }
            if result.mode == "index-only" {
                println!(
                    "  note: native session data was not changed and may be indexed again by a later sync"
                );
            }
        }
        SessionActionFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "session": session_ref_json(&session),
                    "delete": result,
                    "dry_run": dry_run
                }))?
            );
        }
    }
    Ok(())
}

'''
replace_once("src/session.rs", "fn cmd_session_handoff(\n", delete_fn + "fn cmd_session_handoff(\n")

readme = Path("README.md")
text = readme.read_text()
line = "recall session share --id <session-id> --format json  # publish one selected session\n"
addition = "recall session delete --id <session-id> --dry-run  # preview safe session deletion\n"
if addition not in text:
    if line not in text:
        raise SystemExit("README.md usage marker not found")
    readme.write_text(text.replace(line, line + addition, 1))

readme_zh = Path("README.zh-CN.md")
text = readme_zh.read_text()
line = "recall session share --id <session-id> --format json  # 发布选中的一个会话\n"
addition = "recall session delete --id <session-id> --dry-run  # 预览安全删除会话\n"
if addition not in text:
    if line not in text:
        raise SystemExit("README.zh-CN.md usage marker not found")
    readme_zh.write_text(text.replace(line, line + addition, 1))
