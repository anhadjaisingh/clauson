# CLAUDE.md - Clauson Project Conventions

## CLI Naming Conventions

| Principle | Rule |
|-----------|------|
| **Noun-verb hierarchy** | Top-level = plural nouns (`blocks`, `turns`, `stats`). Second-level = verbs/actions (`list`, `show`, `summary`). |
| **Subcommand = different output shape** | Use separate subcommands when the operation fundamentally changes output structure (table vs distribution vs sample list). |
| **Flag = parameter within an operation** | Use flags for metric choice (`--metric`), grouping dimension (`--group-by`), and filtering (`--tool`, `--turn`). |
| **Consistent flag names** | Same flag name means same thing everywhere. `--tool` always filters by tool name. `--metric` always selects the measurement. |
| **Sensible defaults** | Bare `stats` = `summary`. Default metric = `tokens`. Default grouping = `tool`. |
| **Kebab-case for multi-word** | `--group-by`, `tool-calls` for flag values and subcommand names. |

## Build & Test

```bash
cargo clippy -- -D warnings   # Lint
cargo build                    # Build
cargo test                     # All tests
```

## Project Structure

- `src/cli/` - CLI command implementations (one file per top-level noun)
- `src/model/` - Data model (Block, Session, Turn, TokenUsage)
- `src/parser/` - JSONL session file parser
- `tests/cli_tests.rs` - Integration tests using assert_cmd
