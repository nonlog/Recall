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

Implement these two TUI changes:

1. Codex topology filter / subagent visibility
   - Default view should hide Codex subagent sessions and show normal primary sessions.
   - Add a clear TUI mode/filter with `Primary`, `Subagents`, and `All` states.
   - Preserve existing project/source/time filters and bulk selection behavior.
   - Prefer topology metadata (`ThreadRole::Primary` / `ThreadRole::Subagent`) rather than title heuristics.

2. Agent/harness brand icons
   - Keep the existing brand colors.
   - Add a compact brand icon/glyph for each supported source where a stable terminal/Nerd Font glyph exists.
   - Provide a readable Unicode/lettermark fallback where no reliable glyph exists.
   - Do not attempt inline SVG rendering in the Ratatui cell grid.
   - The UI must remain usable when Nerd Font glyphs are unavailable.

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
