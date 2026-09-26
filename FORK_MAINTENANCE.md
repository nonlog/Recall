# Fork maintenance

This repository is the customized nonlog/Recall fork of samzong/Recall.

FORK_FEATURES.md is the authoritative behavior contract. HANDOFF.md records the
implementation and deployment history.

## Automated upstream synchronization

.github/workflows/sync-upstream.yml runs daily and can also be started manually
from the Actions tab.

The workflow tracks upstream main, because this fork intentionally follows the
latest upstream code rather than waiting only for a GitHub Release.

The workflow:

1. fetches samzong/Recall:main;
2. stops immediately when fork main already contains that upstream commit;
3. attempts a normal Git merge into a dedicated sync/upstream-<sha> review
   branch;
4. commits that merge as Codex <codex@openai.com>;
5. runs the fast executable fork-contract preflight;
6. pushes only the review branch and opens or refreshes a PR against fork main;
7. relies on normal PR CI (make check) for the complete Rust audit, format,
   clippy, tests, and fork contract.

It never auto-merges the PR, never resets fork main to upstream, and never
creates a release tag. Release and Scoop publication remain a separate,
deliberate step after the sync PR has been reviewed.

Repository settings required by this workflow are enabled on nonlog/Recall:
GitHub Issues are enabled for conflict reports, and GitHub Actions is allowed to
create pull requests. The workflow itself still requests only the contents,
pull-requests, and issues permissions it needs.

## Conflict and regression behavior

Synchronization is fail-closed.

If Git reports merge conflicts, the workflow aborts the merge, leaves fork main
unchanged, and creates or updates an issue labeled upstream-sync and
needs-fork-review with the conflicting files.

If Git merges cleanly but scripts/check-fork-contract.sh fails, the merge
candidate is still pushed for inspection, but its PR is opened as a draft,
labeled needs-fork-review, and the sync workflow fails. This catches semantic
fork regressions that do not produce textual merge conflicts.

Normal CI also runs the contract checker through make check, so manually
written PRs and release tags cannot silently bypass the fork contract.

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

Then open a PR to nonlog/Recall:main and review it against FORK_FEATURES.md and
HANDOFF.md.

Do not use git reset --hard upstream/main, .gitattributes merge=ours, or a
blanket -X ours merge strategy. Those approaches can silently discard upstream
fixes or fork safety behavior.
