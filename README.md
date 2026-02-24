# clauson

A fast CLI for analyzing Claude Code session files. Parses JSONL session logs into a typed block DAG and lets you query tokens, time, tools, and turns.

## Install

```bash
cargo install --path .
```

Or run directly without installing:

```bash
cargo run -- <file> stats summary
```

Session files live at `~/.claude/projects/<project-hash>/sessions/<session-id>.jsonl`.

## Examples

### Where did my tokens go?

```
$ clauson session.jsonl stats summary

Tool Name    Input  Output  Cache Create  Cache Read      Total  % of Total
---------------------------------------------------------------------------
Bash            31   1,276        16,658   1,636,108  1,654,073       32.3%
Read            25     739        38,521   1,026,178  1,065,463       20.8%
Write           17   3,910        19,394     829,703    853,024       16.7%
Glob             8      51        30,724     322,282    353,065        6.9%
Edit             5     298         5,797     261,371    267,471        5.2%
...
```

### What Bash commands took the most time?

Use `--tool` to drill down into a specific tool's invocations:

```
$ clauson session.jsonl stats summary --metric time --tool Bash

Detail                                                        Count   Total     Avg      %
------------------------------------------------------------------------------------------
node --input-type=module -e "                                     1  232.7s  232.7s  71.3%
npx vitest run src/server/__tests__/watcher.test.ts 2>&1 ...      6   35.5s    5.9s  10.9%
npx vitest run src/server/__tests__/cli.test.ts 2>&1 | ta...      2    8.5s    4.3s   2.6%
mkdir -p /Users/anhad/Projects/claude-tracer/src/server/_...      1    6.9s    6.9s   2.1%
git push origin main                                              1    6.6s    6.6s   2.0%
...
```

### Is tool usage spiky or consistent?

```
$ clauson session.jsonl stats distribution --metric time

Tool Name    Count    Min     Max   Mean  Median   p90     p99
--------------------------------------------------------------
Bash            19   2.8s  232.7s  17.2s    5.7s  6.9s  192.1s
Read            16   56ms   11.3s   3.0s    3.1s  4.3s   10.2s
Write           10   2.5s    4.7s   3.2s    3.0s  4.0s    4.6s
Grep             2   3.0s    8.1s   5.6s    5.6s  7.6s    8.1s
...
```

### What does a heavy turn look like?

```
$ clauson session.jsonl stats sample

Percentile  Turn      Value  User Prompt
-------------------------------------------------------------------------------
p10            3     70,113  Fix the failing test...
p50            2    208,687  Add WebSocket support to the server...
p90            1  4,844,379  You are building a real-time file watcher...
p99            1  4,844,379  You are building a real-time file watcher...
```

### Quick token total

```
$ clauson session.jsonl stats summary --group-by none

Token Summary
────────────────────────────────────────
  Input tokens:                     117
  Cache creation tokens:        220,477
  Cache read tokens:          4,896,021
  Output tokens:                  6,564
────────────────────────────────────────
  Total:                      5,123,179
```

## Commands

### `stats` — Analytics and statistics

The main command. Three subcommands, each answering a different question:

```bash
# summary: "How much of X went to each Y?"
clauson file.jsonl stats summary                                  # tokens by tool (default)
clauson file.jsonl stats summary --group-by none                  # aggregate token totals
clauson file.jsonl stats summary --group-by turn                  # tokens per turn
clauson file.jsonl stats summary --group-by turn --sort-by tokens # worst turns first
clauson file.jsonl stats summary --group-by type                  # tokens by block type
clauson file.jsonl stats summary --metric time                    # time by tool
clauson file.jsonl stats summary --metric time --group-by type    # time by block type
clauson file.jsonl stats summary --metric tool-calls              # invocation counts
clauson file.jsonl stats summary --metric tool-calls --group-by turn --sort-by tool-calls
clauson file.jsonl stats summary --metric time --tool Bash        # drill into Bash commands

# distribution: "What does the spread look like?"
clauson file.jsonl stats distribution                             # token spread by tool
clauson file.jsonl stats distribution --metric time               # time spread by tool
clauson file.jsonl stats distribution --tool Read                 # spread of Read file paths

# sample: "Show me real examples at percentile boundaries"
clauson file.jsonl stats sample                                   # sample turns by tokens
clauson file.jsonl stats sample --metric time --count 3           # 3 turns near each percentile
clauson file.jsonl stats sample --tool Bash                       # sample individual Bash blocks
```

**Common flags** across all `stats` subcommands:

| Flag | Values | Default | Meaning |
|------|--------|---------|---------|
| `--metric` | `tokens`, `time`, `tool-calls` | `tokens` | What to measure |
| `--group-by` | `tool`, `type`, `turn`, `none` | `tool` | How to group |
| `--tool` | any tool name | — | Filter to one tool; enables drill-down |
| `--token-type` | `total`, `input`, `output`, `cache-read`, `cache-create`, `all`, or comma-separated | `total` | Which token components to show (when `--metric tokens`) |
| `--sort-by` | `tokens`, `time`, `tool-calls` | — | Sort descending (only with `--group-by turn`) |

When `--tool` is specified, clauson extracts the relevant detail from each invocation: the command for Bash, the file path for Read/Write/Edit, the pattern for Grep/Glob.

`--token-type` controls which token components appear. Comma-separate to show multiple:

```bash
clauson file.jsonl stats summary --token-type total               # single Total column (default)
clauson file.jsonl stats summary --token-type output,cache-read   # compare output vs cached side by side
clauson file.jsonl stats summary --token-type all                 # all five columns
clauson file.jsonl stats distribution --token-type input,output   # interleaved columns: Min (Input), Min (Output), ...
clauson file.jsonl stats distribution --tool Bash --token-type cache-read  # cache-read spread per command
```

### `tool-events` — Permission and tool lifecycle analytics

Analyzes tool event data collected by the `clauson-hooks` plugin (see [Plugin Setup](#plugin-setup) below). Answers questions like "which tools cause the most permission prompts?" and "what percentage of tool calls were blocked on permissions?"

```bash
# summary: "Which tools trigger permission prompts?"
clauson file.jsonl tool-events                                # permission stats by tool (default)
clauson file.jsonl tool-events summary --json                 # as JSON

# list: "What happened in chronological order?"
clauson file.jsonl tool-events list                           # all events
clauson file.jsonl tool-events list --tool Bash               # only Bash events
clauson file.jsonl tool-events list --event PermissionRequest # only permission prompts

# timeline: "What was the lifecycle of each tool call?"
clauson file.jsonl tool-events timeline                       # all tool call lifecycles
clauson file.jsonl tool-events timeline --tool Bash           # only Bash lifecycles
```

Example output:

```
$ clauson session.jsonl tool-events

Tool    Calls  Prompted  Prompt%  Denied  Deny%
-------------------------------------------------
Bash        5         3    60.0%       1  20.0%
Read        3         0     0.0%       0   0.0%
Write       2         2   100.0%       0   0.0%

Total: 10 calls, 5 prompted (50.0%), 1 denied (10.0%)
```

```
$ clauson session.jsonl tool-events timeline --tool Bash

Tool Use ID           Tool   Detail          Status              Wait
---------------------------------------------------------------------
toolu_01              Bash   ls -la          auto-approved
toolu_03              Bash   cargo test      prompted->approved  3.9s
toolu_06              Bash   rm -rf /tmp/... prompted->denied    0.9s
toolu_07              Bash   git status      prompted->approved  1.0s
toolu_08              Bash   echo hello      auto-approved
```

### `blocks` — Query and filter blocks

```bash
clauson file.jsonl blocks                           # list all blocks
clauson file.jsonl blocks list --type tool           # filter by type
clauson file.jsonl blocks list --turn 3              # filter by turn
clauson file.jsonl blocks list --tool-name Bash      # filter by tool
clauson file.jsonl blocks count                      # count by type
clauson file.jsonl blocks count --group-by tool      # count by tool
clauson file.jsonl blocks show <uuid-prefix>         # show block details
clauson file.jsonl blocks show <uuid-prefix> --raw   # show original JSONL lines
```

### `tools` — List tools and usage counts

```bash
clauson file.jsonl tools                             # tools sorted by usage
clauson file.jsonl tools --json                      # as JSON
```

### `turns` — View conversation turns

```bash
clauson file.jsonl turns                             # list all turns
clauson file.jsonl turns show 1                      # detailed view of turn 1
```

### Global flags

| Flag | Effect |
|------|--------|
| `--json` | Output as JSON (works with all commands) |
| `--raw` | Show raw JSONL lines (works with `blocks show`) |

## Plugin Setup

The `tool-events` command requires data collected by the `clauson-hooks` Claude Code plugin. The plugin logs tool lifecycle events (PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure) to a `.tool-events.jsonl` sidecar file next to each session transcript.

### Install the plugin

```
/plugin add /path/to/clauson/plugin
```

Once installed, the plugin runs automatically during Claude Code sessions. Events are logged asynchronously (zero latency impact). The sidecar file appears alongside the session file:

```
~/.claude/projects/<hash>/sessions/
  abc123.jsonl                    # session transcript (already exists)
  abc123.tool-events.jsonl        # tool events sidecar (created by plugin)
```

Then analyze with:

```bash
clauson ~/.claude/projects/<hash>/sessions/abc123.jsonl tool-events
```

## Building from source

```bash
git clone https://github.com/anhadjaisingh/clauson.git
cd clauson
cargo build --release
```

To enable the pre-commit hook (clippy + build checks):

```bash
git config core.hooksPath .githooks
```
