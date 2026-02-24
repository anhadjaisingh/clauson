# Tool Events Test Plan

Tests are regression catch-alls: they protect existing functionality from breaking when the codebase is modified in the future. Each test targets a specific behavior that, if broken, would mean the feature no longer works correctly.

## Unit Tests

### Model: `ToolEventKind` display

| Test | What it protects |
|------|-----------------|
| `tool_event_kind_display` | Ensures Display output matches the exact strings used in CLI output and JSON filtering (e.g., "PreToolUse", not "pre_tool_use"). If this breaks, list `--event` filtering and all table output breaks. |

### Model: `ToolCallLifecycle` classification

| Test | What it protects |
|------|-----------------|
| `lifecycle_auto_approved` | A tool call with no PermissionRequest is classified as auto-approved. Protects the summary Prompt% column. |
| `lifecycle_prompted_approved` | A tool call with PermissionRequest + PostToolUse is prompted but not denied. Protects summary and timeline. |
| `lifecycle_prompted_denied` | A tool call with PermissionRequest + PostToolUseFailure is prompted AND denied. Protects Deny% column. |
| `lifecycle_no_completion` | A tool call with PermissionRequest but no completion event is treated as denied (user closed the dialog / session ended). Edge case. |
| `lifecycle_permission_wait_secs` | Wait time between PermissionRequest and completion is computed correctly. Protects timeline wait column. |
| `lifecycle_no_wait_when_auto_approved` | Auto-approved calls return None for wait time. Protects timeline from showing garbage. |

### Model: `build_lifecycles` grouping

| Test | What it protects |
|------|-----------------|
| `build_lifecycles_groups_by_tool_use_id` | Events with the same tool_use_id are grouped into one lifecycle. Core grouping logic. |
| `build_lifecycles_preserves_order` | Lifecycles are returned in the order their first event appeared. Protects timeline ordering. |
| `build_lifecycles_empty_input` | Empty event list returns empty lifecycles. Protects against panics. |

### Parser: `sidecar_path`

| Test | What it protects |
|------|-----------------|
| `sidecar_path_replaces_extension` | `.jsonl` -> `.tool-events.jsonl`. Core path derivation. |
| `sidecar_path_no_extension` | Files without `.jsonl` extension still get the right sidecar name. |

### Parser: `parse_tool_events`

| Test | What it protects |
|------|-----------------|
| `parse_fixture` | The test fixture parses successfully and produces the expected number of events. Protects against deserialization regressions. |

## Integration Tests (CLI)

These test the full CLI binary end-to-end with `assert_cmd`. They use `testdata/test-session.jsonl` (which has a `.tool-events.jsonl` sidecar).

### Error handling

| Test | What it protects |
|------|-----------------|
| `tool_events_missing_sidecar` | When no sidecar exists, exits non-zero with helpful error message. Protects UX. |

### `tool-events summary`

| Test | What it protects |
|------|-----------------|
| `tool_events_summary_runs` | Bare `tool-events` runs successfully and shows the expected column headers. |
| `tool_events_summary_json_valid` | `--json` produces valid JSON array. Protects machine-readable output. |
| `tool_events_summary_counts_correct` | Total row has correct counts (10 calls, 5 prompted, 1 denied). Protects the core accounting logic. |
| `tool_events_default_is_summary` | Bare `tool-events` produces same output as `tool-events summary`. Protects the default subcommand behavior. |

### `tool-events list`

| Test | What it protects |
|------|-----------------|
| `tool_events_list_runs` | List subcommand runs and shows expected event types. |
| `tool_events_list_filter_tool` | `--tool Read` only returns Read events. Protects tool filtering. |
| `tool_events_list_filter_event` | `--event PermissionRequest` only returns that event type, with correct count. Protects event filtering. |

### `tool-events timeline`

| Test | What it protects |
|------|-----------------|
| `tool_events_timeline_runs` | Timeline subcommand runs and shows status labels. |
| `tool_events_timeline_filter_tool` | `--tool Bash` returns only Bash lifecycles with correct count. |
| `tool_events_timeline_shows_denied` | At least one lifecycle has "prompted->denied" status. Protects denial detection. |
| `tool_events_timeline_json_has_wait` | JSON output includes wait_secs field. Protects wait time computation in output. |
