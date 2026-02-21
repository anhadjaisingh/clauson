# clauson

A CLI for analyzing Claude Code session JSONL files. Parses raw session logs into a typed block DAG and exposes querying via noun-verb subcommands.

## Quickstart

```bash
# Install
cargo install --path .

# Point it at any Claude Code session file
clauson ~/.claude/projects/*/sessions/*.jsonl blocks

# See token usage
clauson session.jsonl tokens summary

# List tools used
clauson session.jsonl tools

# View conversation turns
clauson session.jsonl turns
```

Session files live at `~/.claude/projects/<project-hash>/sessions/<session-id>.jsonl`.

## Commands

### `blocks` - Query and filter blocks

```bash
clauson file.jsonl blocks                          # List all blocks
clauson file.jsonl blocks list --type tool          # Filter by type (user, assistant, tool, system)
clauson file.jsonl blocks list --turn 3             # Filter by turn number
clauson file.jsonl blocks list --tool-name Bash     # Filter by tool name
clauson file.jsonl blocks count                     # Count by type
clauson file.jsonl blocks count --group-by tool     # Count by tool
clauson file.jsonl blocks show <uuid-prefix>        # Show block details
clauson file.jsonl blocks show <uuid-prefix> --raw  # Show original JSONL lines
```

### `tools` - List and analyze tool usage

```bash
clauson file.jsonl tools              # List tools sorted by usage count
clauson file.jsonl tools --json       # JSON output
```

### `tokens` - View token usage statistics

```bash
clauson file.jsonl tokens summary     # Aggregate token counts
clauson file.jsonl tokens by-turn     # Per-turn breakdown
```

### `turns` - View conversation turns

```bash
clauson file.jsonl turns              # List all turns with timestamps and stats
clauson file.jsonl turns show 1       # Detailed view of turn 1
```

### Global flags

- `--json` - Output as JSON (works with all commands)
- `--raw` - Show raw JSONL lines (works with `blocks show`)

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
