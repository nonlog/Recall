# Fork maintenance

This fork intentionally carries a session-deletion feature on top of upstream
[`samzong/Recall`](https://github.com/samzong/Recall).

## Release model

- `main` is the customized branch. Do not reset it to upstream.
- `.github/workflows/sync-upstream.yml` checks the latest upstream **GitHub Release** once per day and can also be run manually.
- The workflow fetches the upstream release tag into a namespaced remote ref, merges it into customized `main`, runs the full `make check` gate, and verifies that `recall session delete` still exposes `--dry-run`, `--permanent`, and `--index-only`.
- Only after those checks pass does it push `main` and create the same version tag in this fork. For example, upstream `v0.5.3` becomes fork tag `v0.5.3`, but the fork tag points to the merged customized commit rather than the upstream commit.
- The workflow then explicitly dispatches `.github/workflows/release.yml`, which builds this fork's binaries and publishes them as a GitHub Release.

Using the same version tag keeps Scoop manifests simple while still producing binaries that contain the fork-only deletion feature.

## Conflict behavior

The sync is fail-closed. If a future upstream release conflicts with the custom deletion implementation, the workflow aborts the merge, leaves `main` unchanged, creates an issue titled `Upstream sync conflict: <tag>`, and does not create a release tag. Resolve that conflict manually, retain the deletion feature, run `make check`, and then publish the matching release tag.

## Manual equivalent

```bash
git remote add upstream https://github.com/samzong/Recall.git
git fetch --no-tags upstream refs/tags/vX.Y.Z:refs/remotes/upstream-release/vX.Y.Z
git merge --no-edit refs/remotes/upstream-release/vX.Y.Z
make check
cargo run --quiet -- session delete --help
git push origin main
git tag vX.Y.Z
git push origin refs/tags/vX.Y.Z
```

Do not use a hard reset to synchronize this fork; that would discard the custom deletion commits.
