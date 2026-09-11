# Recall fork handoff

Last updated: 2026-09-08
Repository: https://github.com/nonlog/Recall
Upstream baseline: samzong/Recall v0.5.8 (`c78cb5ba50963a509966e2c4b0c38b8369a8da48`)
Fork release: `v0.5.8.5`
Release commit: `28f5c3ba37d88959efd4f15dfa3b088e3aa9f9c1`

## Current fork policy

The runtime baseline is upstream Recall v0.5.8. Keep upstream behavior unless a fork-specific item is listed here or in `FORK_FEATURES.md`. `FORK_FEATURES.md` is the mandatory preservation checklist for every upstream merge/rebase. The old Primary/Subagents/All TUI filter and custom `oh-my-pi` adapter remain dropped. Native-title compatibility is retained where upstream misses source-native title metadata.

Retained/requested fork behavior:

- Native-aware safe session deletion: Trash, permanent delete, explicit index-only delete, dry-run, TUI bulk selection and one-confirmation deletion. Recall stages index deletion under an IMMEDIATE transaction before native mutation; Pi can safely re-resolve stale indexed paths by unique source-id match in configured Pi session roots.
- Per-source TUI colors/icons with terminal-safe fallbacks.
- Upstream `omp` adapter/source id is authoritative; deletion only adds safe path validation around its indexed session files.
- Windows Trash defaults beside the installed `recall.exe`; Scoop persists the `trash` directory across upgrades. `RECALL_TRASH_DIR` remains an override.
- Upstream bottom shortcut bar is preserved exactly. Ctrl+S Settings contains the full keyboard shortcut reference.
- Codex native titles are read from `state_5.sqlite.threads.name` plus latest `session_index.jsonl` rename records; session-index names win when the state DB lags.
- Claude Code titles prefer explicit `custom-title`, otherwise latest `ai-title.aiTitle`; Pi titles use latest `session_info.name`. Metadata parser version bumps must backfill unchanged sessions when these rules change.
- Structured `session_events.summary` payloads are capped at 4096 characters; schema v12 compacts existing oversized rows without deleting transcript/messages/usage/file-history identity fields.
- Windows database defaults to `<recall.exe>/data/recall.db`; `RECALL_DB_PATH` overrides it. Scoop must persist both `trash` and `data`.
- Skill Audit scans shared `.agents`, Claude, Codex, Pi, Gemini, and OpenCode locations and normalizes Windows backslash paths for Skill-read detection.

## 2026-09-11 Codex deletion reconciliation

- LOG reproduced another deletion class on Recall row `7abf6fc1-2a42-4304-90bf-4e61c24bfba0` / Codex thread `01a04b13-631e-7362-aef0-7545d0a508b0`: `codex delete --force` returned exit 1, while the rollout file and Codex `state_5.sqlite.threads` row were both absent afterward. Recall rolled its staged index deletion back because command-failure reconciliation existed only for OpenCode.
- Full LOG audit found 270 Recall-indexed Codex sessions: 166 have both a current rollout and native thread row; 104 have neither. There were zero relocated matches and zero duplicate current rollout ids. These 104 historical stale rows explain why deletion failures could recur long after the Pi-specific fix.
- The Codex deletion path now validates/re-resolves rollouts within the available `~/.codex/sessions` and `~/.codex/archived_sessions` roots, requires the native Codex thread registry to confirm missing ids before stale-index cleanup, and reconciles nonzero native delete exits against both surfaces. A failed native command may remove the same already-backed-up validated rollout directly only when Codex itself no longer tracks the id; otherwise deletion remains fail-closed.
- The local `D:\Workspace\Recall` repository was not modified; implementation is being developed and validated through GitHub/Actions, with LOG used only for runtime audit/smoke tests.
- Post-v0.5.8.6 full dry-run found 103/104 stale Codex rows passing. The only remaining row (Recall `18c838c0-134a-4cb8-8ca3-98e6216034bc`, Codex `019e97b7-93b3-7992-b134-9c850f550e17`) has `source_file_path = NULL`, no current rollout, and no native thread row. Both canonical Codex roots exist. The follow-up permits this legacy pathless case only with a valid UUID, a complete trusted-root scan with zero matches, and explicit native-registry absence.
- A broader normal-Trash dry-run against all 334 LOG rows exposed 23 unsupported/edge cases on v0.5.8.6: Codex 1/270, Copilot Chat 6/6, Copilot CLI 2/2, Grok 12/12, and ZCode 2/2. Claude Code 12/12, Pi 16/16, OMP 9/9, and OpenCode 5/5 already passed. This follow-up therefore adds source-specific safe deletion rather than stopping at the single pathless Codex row.
- On LOG, all six Copilot Chat source files currently exist in canonical VS Code chat stores. Both Copilot CLI historical session paths and all twelve Grok historical paths are absent together with their canonical session roots. Both ZCode rows still exist in `~/.zcode/cli/db/db.sqlite`; ZCode is not on PATH, so Recall uses a verified pre-delete SQLite snapshot plus a constrained native database transaction instead of guessing a CLI command.

## v0.5.8.5 Pi orphan deletion

- Feature branch `fix/pi-orphan-delete-cleanup-20260908` landed through PR #15. Final feature SHA is `117babb246b43d265628f4e59e89dfad5341ec5c`; all feature/style commits use Codex author+committer. GitHub PR CI run `34199666325` passed `make check`.
- LOG diagnosis immediately before release still found exactly five Pi orphan index rows. Their historical paths all lie under the available `C:\Users\www\.pi\agent\sessions` root; the root exists, all 13 current JSONL files were readable, and none of the five source ids had a matching native file.
- v0.5.8.5 therefore makes normal confirmed TUI/CLI deletion clean these verified stale Recall rows without creating an empty Trash entry. It does not generalize automatic index-only fallback to Cursor/Kiro or other unsupported/shared-database sources.
- Release workflow `34199889333` passed the tagged `make check`, Windows x86_64, Linux x86_64, macOS x86_64, macOS aarch64, and publication. Windows ZIP SHA256 is `a715df4847a5fcd371a9563bebb0878b54fb395755ef4e56e6d57f2dc6790448`. Scoop bucket commit `70d0d2a90992c8665be84420b1bd528657199f4d` publishes v0.5.8.5.
- LOG upgraded from v0.5.8.4 to v0.5.8.5 after closing the running Recall TUI process. All five known Pi orphan rows then passed `session delete --dry-run`: exit 0, `native_already_missing=true`, zero native paths, and no index mutation. A text dry-run explicitly reports that only the stale Recall index would be removed. The local `D:\Workspace\Recall` repository remained clean.

## v0.5.8.4 deletion consistency

- Feature commits: `46a44729bae8ef5cf2825506e2bdd869293c11a6` (`fix: make native session deletion consistent`) and `74865b2ea372d643d5fb03b96cae6837116a6876` (`test: cover relocated Pi session headers`), both Codex author+committer. PR #14 passed GitHub CI run `34127526571` and was fast-forwarded to `main`; GitHub records `74865b2ea372d643d5fb03b96cae6837116a6876` as the merge SHA.
- Installed v0.5.8.3 pre-fix audit checked all 22 indexed Pi sessions: all 17 with an existing indexed JSONL generated valid Trash plans; the 5 failures all referenced missing files, and none of those five source ids exists anywhere under the current Pi session root.
- Native-aware deletion now stages Recall-side cleanup under `BEGIN IMMEDIATE` before entering the native phase. SQLite staging failure leaves native data untouched; native failure rolls the pending Recall deletion back; direct Trash moves retain rollback on commit failure.
- Pi deletion validates the indexed JSONL first and can recover a moved path by searching configured Pi session roots for exactly one matching source id. After a complete successful trusted-root scan, zero matches are treated as native data already absent for an explicitly confirmed delete, so Recall cleans the stale index without creating an empty Trash entry. Multiple matches and scan errors fail closed.
- Release tag `v0.5.8.4` points to `ceca382367ba1e8b44fb17aa83d23f3c8b205363`. Release workflow run `34127778865` passed `make check`, Windows x86_64, Linux x86_64, macOS x86_64, macOS aarch64, and publication. Windows asset SHA256: `a6a64f086f8e45720b5334a72c08e95ef9f220aa5a141a28f42882d8a4aeb7ea`.
- Scoop bucket commit `6b675ae50247ac7edb9df55aa3f5d9d0b7b91ac2` publishes 0.5.8.4 and continues to persist both `trash` and `data`. LOG upgraded successfully; `recall --version` remains upstream base `0.5.8`. `current/data` and `current/trash` are persisted junctions, `PRAGMA quick_check=ok`, schema is v12, and the legacy `%APPDATA%\recall\recall.db` remains absent.
- Installed real-session dry-runs resolve native paths for Pi (`~/.pi/agent/sessions/...jsonl`), Claude Code (`~/.claude/projects/...jsonl`), and Codex (`~/.codex/sessions/.../rollout-...jsonl`, with the official `codex delete --force` command). A confirmed orphan Pi row now reports missing/unvalidated native data instead of claiming Pi deletion is unsupported.
- Isolated before/after regression reproduced both reported failures without touching real sessions. On v0.5.8.3, a Pi JSONL moved within the configured Pi root still produced `native deletion is not supported for source pi`; v0.5.8.4 re-resolved the same source id to the moved file and Trash deletion completed with native file absent, Recall row absent, and one Trash manifest.
- The v0.5.8.3 Codex-shim/blocked-DB fixture reproduced the exact inconsistency error with `NativeExists=false`, `IndexRows=1`, and a retained safety backup. On v0.5.8.4 the same forced DB failure occurs before native execution (`NativeExists=true`, `IndexRows=1`, `TrashEntries=0`); after removing the blocker, deletion completed with `NativeExists=false`, `IndexRows=0`, and one Trash backup. All temporary fixtures were removed after validation.

## 2026-09-08 Pi orphan deletion follow-up

- LOG still contained five historical Pi index rows whose recorded JSONL paths no longer exist and whose source ids have no match anywhere under the configured Pi session roots. TUI bulk delete therefore still reported failures in v0.5.8.4 even though there was no native data left to protect.
- This follow-up distinguishes an unsupported source from a verified Pi orphan. Pi root traversal and candidate-file I/O errors now propagate; only a complete successful scan with zero source-id matches, where the indexed path belongs to a currently available configured Pi root, may classify native data as already absent. Unavailable roots and unreadable candidates remain fail-closed.
- For that verified-absent state, a confirmed Trash/permanent delete stages and commits Recall index cleanup without touching native storage or creating an empty Trash directory. The native roots are rechecked inside the staged transaction; if a matching Pi session appears before commit, deletion fails closed and the index rolls back.

## 2026-09-07 deletion consistency investigation

- LOG real native paths were verified as Pi under `C:\Users\www\.pi\agent\sessions\...`, Claude Code under `C:\Users\www\.claude\projects\...`, and Codex rollouts under `C:\Users\www\.codex\sessions\YYYY\MM\DD\rollout-...jsonl`.
- All 22 indexed Pi sessions were dry-run checked on installed v0.5.8.3. Every one of the 17 entries whose indexed JSONL still exists produced a valid Trash plan; the 5 failures all point at missing JSONL files. The old generic `native deletion is not supported for source pi` message therefore conflated a stale/missing indexed path with unsupported deletion.
- Pi deletion now validates the indexed JSONL first, then searches the adapter's configured session roots for a unique matching source id when the indexed path is stale. Multiple matches fail closed; zero matches instruct index-only cleanup only when native data is already gone.
- The native/index inconsistency was structural: `session_delete::execute` previously mutated native state first and only afterward opened a separate Recall deletion transaction. A later SQLite busy/constraint/message-vector failure could therefore leave native data deleted while the index remained.
- The fix stages `delete_session_data_tx` under `BEGIN IMMEDIATE` before entering the native phase. If staging fails, native data is untouched; if native deletion fails, the uncommitted index deletion rolls back; direct Trash moves are still restored if the final commit fails.

## v0.5.8.3 closeout

- Feature commit: `1d29883de0eb8c5d9f5aa72b39ef75a8164b5932` (`fix: restore Claude and Pi native titles`), Codex author+committer. PR #13 passed the required GitHub CI and was fast-forwarded to `main`; GitHub records the same feature SHA as the merge commit.
- Claude Code now uses explicit `custom-title` when present, otherwise the latest `ai-title.aiTitle`; Pi uses the latest `session_info.name`. Both adapters bumped metadata parser version from 1 to 2 so unchanged sessions are backfilled.
- LOG pre-fix diagnosis found 8/8 indexed Claude Code sessions with `ai-title` mismatched and 6/13 indexed Pi sessions with `session_info.name` mismatched.
- `FORK_FEATURES.md` is now the authoritative retained-feature contract. Root `AGENTS.md` requires it and this handoff to be read before every upstream merge/rebase.
- Local required gate passed: Recall core 651/651; extension/CLI suites passed; `rx` 141 passed / 1 ignored; audit, fmt, workspace Clippy `-D warnings`, and workspace tests all passed.
- Release tag `v0.5.8.3` points to `235a0e1f083dfc58a1a60dabc23a88272d1c09b3`. Release workflow run `34038114920` passed check, Windows x86_64, Linux x86_64, macOS x86_64, macOS aarch64, and publication. Windows asset SHA256: `6862bc14723182d51d1e0896e65527eaa844899104793b6ae1bdf9f4346070fb`.
- Scoop bucket commit `eb7f0e95f56d8426334ffd48f79547bea03b8139` publishes 0.5.8.3 and continues to persist both `trash` and `data`. LOG was upgraded to Scoop 0.5.8.3; internal `recall --version` remains upstream base `0.5.8`.
- Installed LOG backfill validation: Claude Code native/generated title mismatches are 0/10 indexed sessions with `custom-title`/`ai-title`; Pi native/generated title mismatches are 0/15 indexed sessions with `session_info.name`. The reported Claude session now resolves to `cua-driver 连接 Claude Code 与 pi`; the reported Pi session resolves to `VS Code 多 Profile 快捷键配置是否同步`; the Showly Pi session resolves to `分析 Showly APK 并复用 Trakt API 凭据`.
- Persisted DB remains `D:\Programs\Scoop\persist\recall\data\recall.db`; `quick_check=ok`; the legacy `%APPDATA%\recall\recall.db` remains absent after installed sync.
- CodSpeed remains the same non-gating external authorization issue: the benchmark suite executes and measures successfully, then upload returns `401 Unauthorized` because `nonlog/Recall` is not authorized in CodSpeed.

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
