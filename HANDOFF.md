# Recall fork handoff

Last updated: 2026-09-03
Repository: https://github.com/nonlog/Recall
Upstream baseline: samzong/Recall v0.5.8 (`c78cb5ba50963a509966e2c4b0c38b8369a8da48`)
Fork release: `v0.5.8.1`
Release commit: `ac78cc5d8570e62c664cc260559d0839f4048a8e`

## Current fork policy

The runtime baseline is upstream Recall v0.5.8. Keep upstream behavior unless a fork-specific item is listed here. The old fork-specific Codex native-title patch, Primary/Subagents/All TUI filter, and custom `oh-my-pi` adapter were intentionally dropped.

Retained/requested fork behavior:

- Native-aware safe session deletion: Trash, permanent delete, explicit index-only delete, dry-run, TUI bulk selection and one-confirmation deletion.
- Per-source TUI colors/icons with terminal-safe fallbacks.
- Upstream `omp` adapter/source id is authoritative; deletion only adds safe path validation around its indexed session files.
- Windows Trash defaults beside the installed `recall.exe`; Scoop persists the `trash` directory across upgrades. `RECALL_TRASH_DIR` remains an override.
- Upstream bottom shortcut bar is preserved exactly. Ctrl+S Settings contains the full keyboard shortcut reference.

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
