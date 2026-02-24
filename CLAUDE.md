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
- `src/model/` - Data model (Block, Session, Turn, TokenUsage, ToolEvent)
- `src/parser/` - JSONL session file parser + tool-events sidecar parser
- `tests/cli_tests.rs` - Integration tests using assert_cmd
- `plugin/` - Claude Code hook plugin for tool event logging

## Testing Conventions

Every new feature or subcommand must include a concise test plan before tests are written. The plan goes in `docs/plans/` and lists:
- Each test name
- What behavior it protects (one sentence)

Tests serve as **regression catch-alls** — they exist to prevent existing features from breaking when the codebase changes. Write tests with that goal in mind:

| Layer | What to test | Where |
|-------|-------------|-------|
| **Unit tests** | Model logic (classification, grouping, computation), parser correctness, path derivation | `#[cfg(test)] mod tests` in the source file |
| **Integration tests** | CLI runs successfully, correct exit codes, output contains expected columns/values, `--json` produces valid JSON, default subcommand matches explicit, filters work | `tests/cli_tests.rs` |

Guidelines:
- Integration tests use `assert_cmd` with the test fixtures in `testdata/`
- For JSON output, parse it and assert on specific field values (not string matching)
- Test both success paths and error paths (e.g., missing files)
- For subcommands with a default, verify bare command equals explicit default
- When adding a new fixture, ensure the expected counts/values are documented in the test plan
