# nonlog fork feature contract

This file is the authoritative checklist of behavior added by `nonlog/Recall` on
top of upstream Recall. These items are intentional product behavior, not
 temporary patches. Do not remove, replace, or silently revert them during an
 upstream merge/rebase unless the user explicitly asks for that specific change.

## Mandatory upstream-merge procedure

Before merging or rebasing a new upstream release:

1. Read this file and `HANDOFF.md` before resolving conflicts.
2. Compare every feature below against the proposed merge tree. An upstream
   implementation may replace a fork implementation only when it preserves the
   same user-visible behavior and data-safety guarantees.
3. Keep the upstream adapter/runtime implementation when possible and re-apply
   fork behavior as small additive patches. Do not revive retired fork adapters
   just to avoid adapting to upstream.
4. Run `make check` before push, then exercise the LOG smoke checks listed below.
5. Update this file and `HANDOFF.md` in the same change whenever a retained fork
   feature is intentionally added, changed, or retired.
6. For commits created by an agent, author and committer must both be
   `Codex <codex@openai.com>`.

## Retained fork features

### Safe session deletion

- `Del` moves native session data to Recall Trash when safe native paths are
  available; `Ctrl+D` permanently deletes.
- CLI supports Trash, permanent, explicit `--index-only`, and `--dry-run`.
- TUI supports bulk selection and one confirmation for the selected set.
- Unsupported/unsafe native deletion must fail closed and require explicit
  index-only deletion rather than pretending native data was removed.
- Pi deletion re-resolves a stale indexed path by source id inside Pi's configured
  session roots. A unique validated match may be trashed; after a complete successful
  trusted-root scan whose indexed path belongs to a currently available root, zero matches
  mean native data is already absent and an explicitly confirmed delete removes the stale
  Recall index only. Multiple matches, unavailable roots, or scan errors
  fail closed instead of guessing.
- Native-aware deletion stages the Recall index deletion inside an IMMEDIATE SQLite
  transaction before native data is touched. SQLite lock/constraint/vector cleanup
  failures therefore happen first; native failure rolls the staged index change back.
- Upstream `omp` remains the authoritative OMP adapter/source id; the fork only
  adds safe deletion/path handling around it.
- Landmarks: `src/session_delete.rs`, `src/session.rs`, `src/tui/app.rs`,
  `src/tui/delete_state.rs`, `src/tui/ui/popups.rs`.

### Persisted Windows Trash and database

- Windows Trash defaults beside `recall.exe` and may be overridden with
  `RECALL_TRASH_DIR`.
- Windows database defaults to `<recall.exe>/data/recall.db` and may be
  overridden with `RECALL_DB_PATH`.
- Scoop manifest must persist both `trash` and `data`, so upgrades do not lose
  deleted-session backups or the Recall index.
- Landmarks: `src/session_delete.rs`, `src/db/store.rs`, `nonlog/scoop-www`'s
  `bucket/recall.json`.

### Database bloat protection

- `session_events.summary` is capped at 4096 characters at write time.
- Schema migration compacts existing oversized summaries without deleting
  transcript messages, usage events, file history, or session identity data.
- Landmarks: `src/db/event_store.rs`, `src/db/schema.rs`.

### Native session titles

Prompt fallback is a last resort. When the source stores a native/generated
session title, Recall must use it and metadata parser versions must be bumped
when title parsing changes so unchanged files backfill correctly.

- Codex: read `state_5.sqlite.threads.name` plus append-only
  `session_index.jsonl.thread_name`; latest session-index rename wins when the
  state DB lags. Existing indexed sessions refresh without requiring rollout
  content changes. Landmark: `src/adapters/codex.rs`.
- Claude Code: use explicit `custom-title` when present; otherwise use the
  latest `ai-title.aiTitle`. Explicit custom title always wins over AI title.
  Landmark: `src/adapters/claude_code.rs`.
- Pi: use the latest `session_info.name` as the native/generated title.
  Landmark: `src/adapters/pi.rs`.

### TUI source branding

- Keep per-source colors/icons with terminal-safe fallbacks.
- `RECALL_ICON_STYLE` remains the user fallback control.
- Landmark: `src/tui/source_brand.rs` and its callers/tests.

### Shortcut UX

- Keep the upstream bottom shortcut bar rather than a fork-only replacement.
- Ctrl+S Settings must contain the complete keyboard shortcut reference,
  including bulk selection, Trash, permanent delete, usage, viewer, and other
  supported modes.
- Landmarks: `src/tui/ui/popups.rs`, `src/tui/ui/mod.rs` tests.

### Skill Audit discovery on Windows/multi-harness setups

- Scan shared `.agents`, Claude, Codex, Pi, Gemini, and OpenCode skill roots.
- Use robust Windows home discovery.
- Normalize Windows backslash paths when identifying `SKILL.md` and references
  activity from indexed events.
- Landmarks: `src/skill_audit.rs`, `src/db/skill_audit_store.rs`.

## Explicitly retired fork behavior

Do not reintroduce these merely because they exist in old commits:

- The old `Primary/Subagents/All` top-level TUI filter.
- The custom `oh-my-pi` adapter/source. Upstream `omp` is authoritative.

## LOG smoke checks after an upstream merge

At minimum, verify on the installed Windows build:

- `scoop info recall` points at the intended fork version.
- `current/data` and `current/trash` are Scoop-persisted junctions.
- `recall info` uses the persisted DB and does not recreate the legacy
  `%APPDATA%/recall/recall.db`.
- A Codex session with a native rename matches Codex metadata.
- A Claude Code session containing `ai-title` displays that title rather than
  `/effort`, `<local-command-stdout>`, or the first prompt.
- A Pi session containing `session_info.name` displays that name rather than
  the first prompt.
- `recall usage` Skill Audit discovers the expected shared/user skill set.
- `Del` performs Trash semantics and `Ctrl+D` is the explicit permanent path.
