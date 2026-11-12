**English** | [Chinese](./README.zh-CN.md)

# Recall

> Local-first search across every AI coding session on your machine.

[![Recall](docs/recall.png)](https://asciinema.org/a/909453)

Jump between Claude Code, Codex, and whatever comes next; Recall pulls those scattered local sessions into one searchable index, tracks usage when token metadata is available, and drops you back into the original CLI.

## Architecture

![Recall architecture](docs/recall-architecture.svg)

## Install

```bash
brew install samzong/tap/recall
```

The Scoop package installs Herdr Agent Icons Max for the current user; configure your terminal to use it (for example, as a fallback font).

## Usage

```bash
recall sync          # incremental sync of the current project (safe to run anytime)
recall sync --project all  # sync every project and run index-wide maintenance
recall               # launch TUI
recall usage         # usage dashboard
recall export --project all > recall-export.jsonl # export all sessions
recall import recall-export.jsonl --dry-run  # preview an import
recall session list  # list sessions for agents/scripts
recall session share --id <session-id> --format json  # publish one selected session
recall session delete --id <session-id> --dry-run  # preview safe session deletion
recall share list --format json  # list published share URLs
recall share unpublish <share-id> --yes  # take down one public page
recall info  # index stats and worker status
```

### Agent icons in terminal UI

Recall defaults to [Herdr Agent Icons Max](https://github.com/hhdebb/herdr-radar) agent logos. Release archives include the patched icon-only font and its license in `fonts/`; the patch adds the Droid logo at U+E1BB. Configure your terminal to render the font, or set `RECALL_ICON_STYLE=nerd` (also `nerd-font`) for Nerd Font marks or `RECALL_ICON_STYLE=plain` for Unicode marks. Unpatched Herdr fonts may not contain the Droid glyph.

### Delete sessions

`recall session delete` removes one selected session from Recall and, when the source has a safe native deletion path, from the source agent as well.

```bash
recall session delete --id <session-id> --dry-run   # preview only
recall session delete --id <session-id>             # safe default: keep a Recall trash backup
recall session delete --id <session-id> --permanent # no Recall trash backup
recall session delete --id <session-id> --index-only # leave source-agent data untouched
```

Codex and OpenCode use their official delete commands so their own metadata stays consistent. Claude Code, Pi, OMP, Antigravity, Gemini, Grok, Copilot CLI, Cline, DeepSeek Harness, Droid, and Kimi Code use validated per-session files or directories. Shared-database sources without a stable native delete API require `--index-only`; Recall deliberately does not guess writes into their databases.

Droid native deletion is **local-only**: Recall validates the UUID and native
`session_start.id` under `~/.factory/sessions` (or `DROID_SESSIONS_DIR`), then
moves/removes only that session's `.jsonl`, `.settings.json`, and
`.settings.json.bak`. Ambiguous or unsafe matches fail closed; use `--dry-run`
to inspect paths before deleting. Droid can mirror sessions to Factory cloud
(`cloudSessionSync` defaults to `true`); deleting local files does **not**
delete cloud copies, which may still appear in Factory or be re-synced.

On Windows, the default trash is the `trash` directory beside the installed `recall.exe`; the Scoop package persists that directory across upgrades. `RECALL_TRASH_DIR` can override the location. On other platforms Recall falls back to its data directory. For sources with an official native delete command, Recall creates a safety backup first; for file-backed sources it moves the native session data into the trash. Imported sessions are always index-only.

With Skill use **Recall** is the best way.

```bash
recall skill install # auto detect agents and install skills
```

## Support

One index across every AI coding CLI. Sync once, search everywhere, resume right where you left off.

| Adapter         | Discovery | Full-index | Incremental-sync | Semantic-search | Export | Resume | Usage |
| --------------- | :-------: | :--------: | :--------------: | :-------------: | :-------------: | :----: | :----: |
| Claude Code     |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| OpenCode        |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Codex           |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Pi              |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| OMP             |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| rx-agent        |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Antigravity |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |      |
| Gemini          |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Kiro            |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   —    |       |
| Copilot     |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Copilot Chat |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   —    |      |
| Cursor          |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   —    |   ✅   |
| Cline           |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   —    |       |
| Roo             |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   —    |       |
| DeepSeek Harness |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   —    |   ✅   |
| Grok            |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Kimi Code       |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| MiniMax Code    |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Qoder CLI       |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Qwen Code       |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Kilo Code       |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Crush           |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| MiMo Code       |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| ZCode           |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Goose           |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Droid           |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Amp             |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |      |
| OpenHands       |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |      |
| Devin           |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Muse Code       |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| CodeBuddy       |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| TRAE CLI        |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| JetBrains Junie |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   ✅   |   ✅   |
| Zed             |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   —    |   ✅   |
| Warp            |     ✅    |     ✅     |        ✅        |        ✅       |        ✅       |   —    |   ✅   |

Qoder CLI usage requires exposed, nonzero token counts. Cache-inclusive input is split into fresh input, cache read, and cache write. Hidden token counts are not estimated.

rx-agent sessions use `$RX_HOME/sessions` (default `~/.rx/sessions`). Recall reads version 1 logs and preserves the complete history across compaction checkpoints.

## Acknowledgements

- Thanks to [tokscale](https://github.com/junhoyeo/tokscale) for the usage dashboard reference and token accounting behavior.
- Thanks to [Ratatui](https://github.com/ratatui/ratatui) and [Crossterm](https://github.com/crossterm-rs/crossterm) for the terminal UI foundation.
- Thanks to [sqlite-vec](https://github.com/asg017/sqlite-vec) and SQLite FTS5 for keeping local text and vector search embedded.
- Thanks to [Candle](https://github.com/huggingface/candle), Hugging Face, and [intfloat/multilingual-e5-small](https://huggingface.co/intfloat/multilingual-e5-small) for local semantic embeddings.
- Thanks to [kitup](https://github.com/samzong/kitup) for the bundled agent skill installer.
- Thanks to [LINUX DO](https://linux.do/) for the open-source sharing community.

## License

This project is licensed under the [MIT](LICENSE) License.
