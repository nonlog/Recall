# recall-calibrate

Official Recall extension that suggests agent settings from your Recall usage history.
It reads Recall through `recall export` and does not change any file.
It requests `usage-ownership` and counts only events that core marks with `is_counted: true`.
If Recall rejects that selector or omits the field, the extension stops and asks you to rebuild or upgrade Recall.

## compact

`recall calibrate compact` suggests the context size at which each agent compacts.
It replays your recorded calls against each candidate threshold and picks the one with the lowest weighted token cost.

```sh
recall calibrate compact
recall calibrate compact --source codex --time 30d
recall calibrate compact --format json
```

Options:

- `--source claude-code|codex` limits the report to one agent. Without it, the report covers both.
- `--time` takes a Recall time window: `today`, `7d`, `week`, `30d`, `month`, or `all`. The default is `7d`.
- `--format text|json` selects the output. JSON goes to stdout. Progress goes to stderr.

The time window applies to usage event timestamps, including calls from sessions that started before the window.
The `calls` in each advice item count only calls with a usable prompt and a supported model.
These advice counts can be lower than `recall usage --json`, which includes all counted usage events.

The report reads the current values from these locations:

| Agent | Setting | File |
| --- | --- | --- |
| Claude Code | `modelSettings.<model>.autoCompactWindow`, then `autoCompactWindow` | `$CLAUDE_CONFIG_DIR/settings.json`, or `~/.claude/settings.json` |
| Codex | `model_auto_compact_token_limit` | `$CODEX_HOME/config.toml`, or `~/.codex/config.toml` |

If `CLAUDE_CODE_AUTO_COMPACT_WINDOW` is set in the environment or in the `env` block of `settings.json`, that value overrides the file. The report then sets `setting.locked_by` to the variable name.

Claude Code gets one result per `modelSettings` key, so dated model IDs such as `claude-haiku-4-5-20251001` count toward `claude-haiku-4-5`. Codex gets one result for all models, because Codex has one top-level setting.

### JSON fields for automation

Each item in `advice` has these fields:

- `agent`, `model`, and `calls` identify the group.
- `setting` gives the file, key, current value, source, and locked environment variable.
- `apply` is the value to write. It is `null` when the current value is already the best value or when the group is skipped.
- `saving` is the share of weighted tokens that the suggested value saves.
- `skipped` gives the reason when the group has no advice.
- `compaction` has the full cost curve and observed facts.

The extension reports values and does not write agent settings. Apply the suggested value manually to the reported file and key.

### Limits

- The cost model uses only token counts. Recall does not record how long a compaction takes, so time is not part of the choice.
- The replay does not measure the quality of work after a compaction.
- The cost weights are fixed ratios, not real prices.
- The report reads only the exact `modelSettings` key for each model.

## Attribution

Thanks to [yetone/magpie](https://github.com/yetone/magpie). The `compact` command started from its compaction tuning.
`src/compact.rs` and the `to_call` function in `src/report.rs` are a port of `internal/tune` from magpie, MIT License, Copyright (c) 2026 yetone.
`THIRD_PARTY_NOTICES` has the full notice. It covers only that code. The rest of this extension is under the MIT License of Recall.
