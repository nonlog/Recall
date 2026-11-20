# Fork maintenance

This repository is the customized nonlog/Recall fork of samzong/Recall.

FORK_FEATURES.md is the authoritative behavior contract. HANDOFF.md records the
implementation and deployment history.

## Automated upstream synchronization

.github/workflows/sync-upstream.yml runs daily and can also be started manually
from the Actions tab.

The workflow tracks upstream main and automatically advances fork main when a
normal merge is clean and every gate passes. It uses a compare-and-swap push
against the checked-out main SHA, so a concurrent main update cannot be
silently overwritten.

The workflow:

1. fetches samzong/Recall:main;
2. stops immediately when fork main already contains that upstream commit;
3. attempts a normal Git merge into a candidate commit;
4. aborts on conflicts and creates or updates a needs-fork-review issue;
5. runs the fast executable fork-contract preflight;
6. runs the complete make check gate inside the sync workflow itself;
7. pushes the validated merge commit directly to fork main;
8. preserves a candidate branch and draft PR for manual recovery when a gate
   fails.

It never resets fork main to upstream and never creates a release tag. Release
and Scoop publication remain a separate, deliberate step after the sync push.

Repository settings required by this workflow are enabled on nonlog/Recall:
GitHub Issues are enabled for conflict reports, and GitHub Actions is allowed to
create pull requests. The workflow itself still requests only the contents,
pull-requests, and issues permissions it needs.

The review-branch and main pushes use a repository-scoped write deploy key
stored as the single-line base64 Actions secret
`UPSTREAM_SYNC_DEPLOY_KEY_B64`. Base64 is intentional: it preserves the exact
OpenSSH private-key bytes across secret transport and avoids newline/encoding
corruption. This is required because `GITHUB_TOKEN` cannot create or update
commits that modify `.github/workflows/*`. Checkout uses
`persist-credentials: false`; the deploy key is decoded and validated only for
the push step, then deleted before the merged candidate runs `make check`. Do
not reuse this key for release or runtime deployment operations.

## Conflict and regression behavior

Synchronization is fail-closed.

If Git reports merge conflicts, the workflow aborts the merge, leaves fork main
unchanged, and creates or updates an issue labeled upstream-sync and
needs-fork-review with the conflicting files.

If Git merges cleanly but scripts/check-fork-contract.sh or make check fails,
the validated main branch is unchanged. The candidate branch is preserved and
its PR is opened as a draft labeled needs-fork-review. A successful candidate
is pushed to main only after every gate passes and the compare-and-swap lease
still matches the main SHA that was checked out.

The sync workflow runs the complete make check itself because pull requests
created by GITHUB_TOKEN do not recursively trigger ordinary pull_request
workflows. Normal CI also runs the contract checker through make check, so
manually written PRs and later pushes/releases cannot silently bypass the fork
contract.

## Fork contract

scripts/check-fork-contract.sh is intentionally fast and static. It verifies
that the major fork landmarks and their regression tests still exist:

- native-aware Trash, permanent, index-only deletion and staged SQLite deletion;
- Windows persisted Recall DB and Trash paths;
- 4096-character event-summary protection;
- Codex, Claude Code, and Pi native-title handling;
- per-source branding and hidden empty host labels;
- Ctrl+S Sync / Ctrl+P Settings plus fork delete shortcuts;
- Windows and multi-harness Skill Audit support.

The normal Rust test suite remains the semantic verification layer. The static
contract checker is an additional early warning when upstream reorganizes or
removes one of the fork-specific paths.

## Manual equivalent

    git remote add upstream https://github.com/samzong/Recall.git
    git fetch --no-tags upstream main
    git checkout -b sync/upstream-<sha> origin/main
    git merge --no-ff upstream/main
    ./scripts/check-fork-contract.sh
    make check
    git push origin HEAD

Then inspect the candidate against `FORK_FEATURES.md` and `HANDOFF.md`. A
clean candidate is pushed to `main` automatically by the workflow after the
same gates pass.

Do not use git reset --hard upstream/main, .gitattributes merge=ours, or a
blanket -X ours merge strategy. Those approaches can silently discard upstream
fixes or fork safety behavior.
