#!/usr/bin/env bash
set -euo pipefail

failures=0

pass() {
  printf '  [ok] %s\n' "$1"
}

fail() {
  printf '  [FAIL] %s\n' "$1" >&2
  failures=$((failures + 1))
}

require_file() {
  local path="$1"
  if [[ -f "$path" ]]; then
    pass "$path exists"
  else
    fail "$path is missing"
  fi
}

require_text() {
  local path="$1"
  local text="$2"
  local label="$3"
  if [[ -f "$path" ]] && grep -Fq -- "$text" "$path"; then
    pass "$label"
  else
    fail "$label"
  fi
}

printf 'Checking nonlog/Recall fork contract...\n'

require_file "FORK_FEATURES.md"
require_file "HANDOFF.md"
require_file "FORK_MAINTENANCE.md"
require_file ".github/workflows/sync-upstream.yml"
require_file "scripts/check-fork-contract.sh"

require_file "src/session_delete.rs"
require_text "src/session_delete.rs" "pub(crate) enum DeleteMode" "delete modes remain implemented"
require_text "src/session_delete.rs" "TransactionBehavior::Immediate" "Recall deletion stages under an IMMEDIATE transaction"
require_text "src/session_delete.rs" "recall_index_delete_is_staged_before_native_trash_move" "transaction-order regression test remains present"
require_text "src/session_delete.rs" "unsupported_native_delete_does_not_touch_index" "fail-closed deletion regression test remains present"
require_text "src/session.rs" "SessionCommands::Delete" "session delete CLI dispatch remains present"
require_text "src/cli/session_args.rs" "permanent: bool" "session delete --permanent remains present"
require_text "src/cli/session_args.rs" "index_only: bool" "session delete --index-only remains present"
require_text "src/cli/session_args.rs" "dry_run: bool" "session delete --dry-run remains present"
require_text "src/tui/app.rs" "KeyCode::Delete" "TUI Delete-key Trash path remains present"
require_text "src/tui/app.rs" "DeleteMode::Trash" "TUI Trash mode remains present"
require_text "src/tui/app.rs" "DeleteMode::Permanent" "TUI permanent-delete mode remains present"
require_text "src/tui/app.rs" "ctrl_d_from_search_opens_permanent_confirmation" "TUI permanent-delete regression test remains present"

require_text "src/db/store.rs" "RECALL_DB_PATH" "RECALL_DB_PATH override remains present"
require_text "src/session_delete.rs" "RECALL_TRASH_DIR" "RECALL_TRASH_DIR override remains present"
require_text "src/db/event_store.rs" "EVENT_SUMMARY_CHAR_CAP: usize = 4_096" "event summary cap remains 4096 characters"
require_text "src/db/event_store.rs" "event_summary_is_capped_on_unicode_boundaries" "event summary cap regression test remains present"
require_text "src/db/schema.rs" "SET summary = substr(summary, 1, 4096)" "legacy summary compaction remains present"

require_text "src/adapters/codex.rs" "state_5.sqlite" "Codex state DB native titles remain supported"
require_text "src/adapters/codex.rs" "thread_name" "Codex session_index rename titles remain supported"
require_text "src/adapters/claude_code.rs" '"custom-title"' "Claude explicit custom titles remain supported"
require_text "src/adapters/claude_code.rs" '"ai-title"' "Claude AI titles remain supported"
require_text "src/adapters/claude_code.rs" "parse_claude_session_prefers_explicit_title_over_later_ai_title" "Claude title precedence regression test remains present"
require_text "src/adapters/pi_session.rs" '"session_info" if format == Format::Pi' "Pi session_info titles remain supported"

require_file "src/tui/source_brand.rs"
require_text "src/tui/source_brand.rs" "RECALL_ICON_STYLE" "source icon fallback remains configurable"
require_text "src/tui/ui/search.rs" "if !session.locations.is_empty()" "empty host/location metadata remains hidden"
require_text "src/tui/ui/search.rs" "session_metadata_prefix_hides_missing_host" "missing-host regression test remains present"
require_text "src/tui/app.rs" "ctrl_s_from_search_requests_sync_instead_of_settings" "Ctrl+S Sync regression test remains present"
require_text "src/tui/app.rs" "ctrl_p_from_search_opens_settings" "Ctrl+P Settings regression test remains present"
require_text "src/tui/ui/mod.rs" '"Ctrl+S sync"' "shortcut bar advertises Ctrl+S Sync"
require_text "src/tui/ui/mod.rs" '"Ctrl+P settings"' "shortcut bar advertises Ctrl+P Settings"

require_text "src/skill_audit.rs" '.agents/skills' "shared .agents skills remain discoverable"
require_text "src/skill_audit.rs" '.gemini/skills' "Gemini skills remain discoverable"
require_text "src/skill_audit.rs" '.config/opencode/skills' "OpenCode skills remain discoverable"
require_text "src/skill_audit.rs" "skill_id_from_windows_path_reads_skill_md" "Windows skill-path regression test remains present"
require_text "src/db/skill_audit_store.rs" "REPLACE(e.target, char(92), '/')" "indexed Windows skill paths remain normalized"

if (( failures > 0 )); then
  printf '\nFork contract FAILED with %d missing invariant(s).\n' "$failures" >&2
  exit 1
fi

printf '\nFork contract passed.\n'
