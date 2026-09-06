# Recall fork handoff

Last updated: 2026-09-06
Repository: https://github.com/nonlog/Recall
Upstream baseline: samzong/Recall v0.5.8 (`c78cb5ba50963a509966e2c4b0c38b8369a8da48`)
Fork release: `v0.5.8.2`
Release commit: `0b9084472bb3a3b6bea505ce36418e84ef75f247`

## Current fork policy

The runtime baseline is upstream Recall v0.5.8. Keep upstream behavior unless a fork-specific item is listed here. The old Primary/Subagents/All TUI filter and custom `oh-my-pi` adapter remain dropped. Codex native-title compatibility is retained because upstream does not consume Codex native title stores.

Retained/requested fork behavior:

- Native-aware safe session deletion: Trash, permanent delete, explicit index-only delete, dry-run, TUI bulk selection and one-confirmation deletion.
- Per-source TUI colors/icons with terminal-safe fallbacks.
- Upstream `omp` adapter/source id is authoritative; deletion only adds safe path validation around its indexed session files.
- Windows Trash defaults beside the installed `recall.exe`; Scoop persists the `trash` directory across upgrades. `RECALL_TRASH_DIR` remains an override.
- Upstream bottom shortcut bar is preserved exactly. Ctrl+S Settings contains the full keyboard shortcut reference.
- Codex native titles are read from `state_5.sqlite.threads.name` plus latest `session_index.jsonl` rename records; session-index names win when the state DB lags.
- Structured `session_events.summary` payloads are capped at 4096 characters; schema v12 compacts existing oversized rows without deleting transcript/messages/usage/file-history identity fields.
- Windows database defaults to `<recall.exe>/data/recall.db`; `RECALL_DB_PATH` overrides it. Scoop must persist both `trash` and `data`.
- Skill Audit scans shared `.agents`, Claude, Codex, Pi, Gemini, and OpenCode locations and normalizes Windows backslash paths for Skill-read detection.

## v0.5.8.2 closeout

- Feature commit: `0a10a89bcd2da472e98a1482c1b8b1ff4cc7bb54` (`fix: compact index and restore native metadata`), Codex author+committer. PR #12 passed the required GitHub CI and was fast-forwarded to `main`; GitHub records the same feature SHA as the merge commit.
- Local required gate: `make check` passed. Recall core was 647/647; extension/CLI suites passed; `rx` was 141 passed / 1 ignored. Audit, fmt, workspace Clippy `-D warnings`, and workspace tests all passed.
- Release tag `v0.5.8.2` points to `0b9084472bb3a3b6bea505ce36418e84ef75f247`. Release workflow run `34035105043` passed check, Windows x86_64, Linux x86_64, macOS x86_64, macOS aarch64, and publication.
- Windows release asset SHA256: `b48f6f7068b5f8e0d6c1c85c261375e16b3761185da2aa070e44cd8506d1bfda`.
- Scoop bucket commit: `cdecf2f491a4daf5eeb8bc9c0ed499a5823831fc`; manifest version 0.5.8.2 persists both `trash` and `data`.
- LOG is installed on Scoop 0.5.8.2 (`recall --version` remains upstream base `0.5.8`). `current/data` -> `D:\Programs\Scoop\persist\recall\data`; `current/trash` -> `D:\Programs\Scoop\persist\recall\trash`.
- The legacy `%APPDATA%\recall\recall.db` was migrated and removed only after the persisted database passed integrity/count checks. Final persisted DB: 558,202,880 bytes (~532.34 MiB), down 74.15% from the 2,158,985,216-byte legacy DB. Final rows: 355 sessions, 61,192 messages, 81,227 usage events, 141,936 session events; schema v12; zero summaries over 4096 chars; `quick_check=ok`. `recall info` after removal did not recreate the legacy DB.
- Installed Codex sync restored native titles without rollout reparsing. For 84 indexed sessions that also had a native title from `state_5.sqlite` / latest `session_index.jsonl`, mismatches were 0. The reported DTU session `01a055bf-e77a-7d01-8516-913eca321720` now resolves to `选择4G DTU开发工具 (2)`.
- Installed Skill Audit reports 86 installed skills, 19 occasional and 67 dormant for the current default range. Windows-path normalization expanded real indexed skill activity from 385 old-path matches to 2,215 matches (+1,830 backslash-only events).
- `recall usage --json --time 7d` still works after migration; the checked report contained 63 sessions / 4,318 usage events and 492,800,640 total tokens, confirming usage data survived the move.
- CodSpeed remains a non-gating external integration issue: benchmarks execute successfully, then upload returns 401 because `nonlog/Recall` is not authorized in CodSpeed.

## Release / validation

- PR #11 was validated by GitHub CI and merged by fast-forwarding a Codex-authored merge commit whose first parent is the prior fork `main` and whose tree is the validated upstream-v0.5.8-based feature tree.
- Feature commit: `6a97d8beccd109858a5f3aad6a252c1682b3c24e` (Codex author+committer).
- Release-prep commit: `9c3227c7616eba7cfb3038ec61f1734ad2cdc98d` (Codex author+committer).
- Merge/release commit: `ac78cc5d8570e62c664cc260559d0839f4048a8e` (Codex author+committer).
- Local Linux validation: `cargo test --workspace` passed; Recall core 640/640. `cargo clippy --workspace --all-targets --features bench -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check` passed.
- GitHub Release run `33742472113` passed release check plus Windows x86_64, Linux x86_64, macOS x86_64, macOS aarch64 builds and publication.
- Windows release asset SHA256: `57d1c32f04edc03fc0b3b8ccbe8bb8d5958f532904c66bb04cbd197c9bb90e2c`.
- CodSpeed still has the pre-existing external integration problem: all benchmarks execute, then result upload fails with `401 Unauthorized` because `nonlog/Recall` is not authorized in CodSpeed. It is not a release/merge gate.

## Scoop / LOG

- `nonlog/scoop-www` commit `8ffd38489b9e8be9de6de704c1d41cff442d8052` publishes `recall` 0.5.8.1 with `persist: "trash"` and rx aliases `rxc`, `rxx`, `rxo`, `rxp`, `rxd`, `rxk`.
- LOG Scoop install is 0.5.8.1; internal binaries correctly report upstream base version `recall 0.5.8` / `rx 0.5.8`.
- `D:\Programs\Scoop\apps\recall\current\trash` is a Junction to `D:\Programs\Scoop\persist\recall\trash`.
- One legacy Trash entry (51,084 bytes) was migrated from `%APPDATA%\recall\trash`; the legacy path was removed only after successful migration.
- Installed full sync registered the upstream adapter set. Examples with local data: `OMP (omp)=8`, `copilot-chat=6`, `zcode=2`.
- The eight legacy `oh-my-pi` index rows had exactly the same source-id set as upstream `omp`; they were removed index-only after verification. Native OMP JSONL/artifact data was not deleted.
- Installed OMP Trash dry-run resolved both the upstream OMP JSONL and its same-stem artifact directory and made no changes.

## Remaining external issue

Configure repository authorization for `nonlog/Recall` in CodSpeed if benchmark result uploads are desired. No Recall code change is currently required for this.
