# Recall Fork Handoff

Last updated: 2026-08-28
Workspace: `D:\Workspace\Recall`
Repository: `https://github.com/nonlog/Recall`
Current baseline: `main` at fork release `v0.5.3.6`

## Purpose of this fork

This fork is intentionally customized for the local multi-agent workflow rather than kept as a minimal upstream patchset. The main additions are safe native session deletion, bulk TUI session management, better native titles, branded source presentation, and additional harness support.

## Completed fork work

- Added `recall session delete` with Trash, permanent, dry-run, and index-only modes.
- Added TUI multi-select and bulk delete:
  - `Space` / `Insert`: toggle selection.
  - `Delete`: Trash selected/current sessions after one confirmation.
  - `Ctrl+D`: permanently delete selected/current sessions after one confirmation.
- Native deletion is verified instead of treating index removal as success.
- Added or hardened native deletion for Codex, Claude Code, Pi, Oh My Pi, Antigravity/Agy, OpenCode, and file-backed adapters.
- Added Oh My Pi (`OMP`) as a distinct source.
- Restored native titles where available for Codex, Claude Code, Pi, OMP, and related adapters; command-only sessions get deterministic fallback titles.
- Added branded source colors in the TUI.
- OpenCode maintenance commands use `--pure`; Trash export is written directly to disk and validated.
- OpenCode stale/ghost Recall rows are pruned when the native OpenCode DB no longer contains the session.
- Automatic upstream release syncing was deliberately disabled in favor of manual integration because this fork now has substantial custom behavior.

## Current task

Implementation is complete locally and awaiting full validation for these two TUI changes:

1. Codex topology filter / subagent visibility
   - Added TUI `Primary`, `Subagents`, and `All` topology modes.
   - Default is `Primary`; internally it uses a `TopLevel` SQL predicate that excludes `subagent` while retaining `NULL`/unclassified roles so Claude/Pi/other sources do not disappear.
   - `Subagents` shows only persisted `thread_role = 'subagent'`; `All` applies no role restriction.
   - Added `R:[Primary|Subagents|All]` to the search header and a `Thread Role` row to the Filters popup.
   - Empty-query recent results now use the same SQL topology filter as text/semantic search rather than the old reachable-subagent collapsing heuristic.

2. Agent/harness brand icons
   - Existing brand colors are retained.
   - Added stable Nerd Font glyphs where a dependable glyph exists (OpenCode/code, OMP/terminal, Antigravity/rocket, Gemini/Google, Copilot, Cline/terminal, Kimi/moon).
   - Sources without a dependable cross-version brand glyph keep recognizable Unicode marks (Claude, Codex, Pi, Grok, Kiro, Cursor, DeepSeek).
   - `RECALL_ICON_STYLE=plain` (also `unicode`, `ascii`, `off`, or `0`) forces fallback marks if the terminal font lacks Nerd Font glyphs.
   - SVG is intentionally not used because Ratatui renders terminal cells rather than inline vector images.

Validation status (2026-08-28):

- `cargo fmt --all -- --check`: passed.
- Targeted topology/source-brand/TUI/database tests: all passed.
- `cargo clippy -p recall --lib --features bench -- -D warnings`: passed, confirming production Recall code is warning-free on Windows.
- Windows all-target Clippy is blocked only by pre-existing platform-specific test warnings in `src/extension.rs` and `crates/rx/src/update.rs`.
- Windows `cargo test --workspace`: 454 passed; the same 3 pre-existing Windows-only tests fail (Cursor temp directory inference, Kimi temp-file PermissionDenied, Pi unavailable-cwd fallback). No new tests fail.
- Real debug TUI smoke test from `D:\Workspace\general`: header shows `R:[Primary]`; the visible list contains top-level CDX/Claude sessions and no Codex `↳` subagents; brand marks and colors render correctly in the terminal cell grid.
- Real indexed-data count through the new SQL filter: 288 total sessions = 186 top-level + 102 subagents; all 102 currently classified subagents are Codex sessions.
- Full Linux `make check` is still required in GitHub CI before merge.

Next: commit feature changes, push PR, wait for CI, merge, publish the next fork patch release, update `www/recall`, and upgrade the log machine when no installed Recall process is holding the executable.

## Important implementation rules

- Read `AGENTS.md` and `src/tui/AGENTS.md` before TUI changes.
- TUI state transitions belong in `app.rs`/state modules; input belongs in `event.rs`; `ui/` is render-only.
- Long-running work must stay off the TUI event loop.
- `make check` must pass before push; GitHub CI uses the same gate.
- Do not commit `.local/` scratch data.
- GitHub commits must use the official Codex identity: `Codex <codex@openai.com>`.
- Update this handoff after each major implementation or validation milestone, and before ending a long context window.

## Validation already established

- OpenCode stale-session failure was reproduced as `Session not found` while a stale Recall row still existed; fixed by native DB-aware pruning.
- Codex subagents are real persisted threads (`thread_source=subagent`) and are indexed with topology metadata.
- Codex official deletion supports persisted threads and performs reference-safety checks; Recall should continue using the native Codex delete path rather than mutating Codex SQLite directly.

## Release/install workflow

Fork releases use tags such as `v0.5.3.6`; the internal Cargo version may remain at the upstream base version. Windows is distributed through the `www` Scoop bucket as `www/recall`.

Typical final validation and deployment sequence:

```powershell
make check
git push origin <feature-branch>
# merge PR after CI
# create fork patch tag
scoop update recall
```
