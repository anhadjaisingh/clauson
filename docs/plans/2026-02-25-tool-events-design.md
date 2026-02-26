# Tool Events: Permission & Lifecycle Tracking for Claude Code Sessions

## Problem

Claude Code session JSONL files don't log permission events. When a tool call requires user approval, the JSONL records the `tool_use` and `tool_result`, but the permission prompt itself is invisible. We can't determine from session data alone whether a prompt was shown, whether it was auto-approved, or how long the user took to respond.

## Solution

Two components:

1. **Claude Code plugin** (`clauson-hooks`) that logs tool lifecycle events to a sidecar JSONL file
2. **`clauson tool-events` subcommand** that analyzes the sidecar data

## Component 1: Hook Plugin

### Structure

```
plugin/
  plugin.json
  hooks/
    hooks.json
    log-tool-event.sh
```

### Hook Events (all async)

| Event | When | What it captures |
|-------|------|-----------------|
| PreToolUse | Every tool call, before execution | Tool name, input, baseline timestamp |
| PermissionRequest | When permission dialog fires | Permission suggestions, prompt timestamp |
| PostToolUse | Tool succeeds | Completion timestamp |
| PostToolUseFailure | Tool fails | Failure timestamp |

All hooks use `"async": true` since they are logging-only and must not add latency.

### `hooks.json`

```json
{
  "hooks": {
    "PreToolUse": [{ "matcher": "", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/log-tool-event.sh", "async": true }] }],
    "PermissionRequest": [{ "matcher": "", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/log-tool-event.sh", "async": true }] }],
    "PostToolUse": [{ "matcher": "", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/log-tool-event.sh", "async": true }] }],
    "PostToolUseFailure": [{ "matcher": "", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/log-tool-event.sh", "async": true }] }]
  }
}
```

### `log-tool-event.sh`

Reads JSON from stdin. Writes one JSONL line to `<transcript>.tool-events.jsonl` (sibling of the session transcript).

Fields written per event:
- `event` (string): hook event name
- `tool_name` (string): tool being called
- `tool_use_id` (string | null): correlates events for one tool call. **Note: Claude Code sends `null` for `PermissionRequest` events** (see Lifecycle Correlation below).
- `tool_input` (object): full tool input
- `session_id` (string): session identifier
- `permission_mode` (string): current permission mode
- `timestamp` (string): ISO 8601
- `permission_suggestions` (array of objects | null, PermissionRequest only): auto-allow suggestions shown to user. Each suggestion is an object with `type`, `rules`, `behavior`, `destination` fields — **not** a plain string array.

#### Real `PermissionRequest` event example (from production data)

```json
{
  "event": "PermissionRequest",
  "tool_name": "Bash",
  "tool_use_id": null,
  "tool_input": {
    "command": "ls -la /some/path",
    "description": "List files"
  },
  "session_id": "d3f6edaa-...",
  "permission_mode": "default",
  "timestamp": "2026-02-26T07:55:21.000Z",
  "permission_suggestions": [
    {
      "type": "addRules",
      "rules": [{ "toolName": "Bash", "ruleContent": "ls*" }],
      "behavior": "allow",
      "destination": "session"
    }
  ]
}
```

### Sidecar File Location

Derived from `transcript_path` in the hook input:
- Session: `~/.claude/projects/<hash>/<id>.jsonl`
- Sidecar: `~/.claude/projects/<hash>/<id>.tool-events.jsonl`

### Tool Call Lifecycle Correlation

**Original assumption (incorrect):**

> Events for one tool call share a `tool_use_id`:
> `PreToolUse(toolu_01) -> PermissionRequest(toolu_01)? -> PostToolUse(toolu_01)`

**Actual behavior:** Claude Code sends `tool_use_id: null` for `PermissionRequest` events. Only `PreToolUse`, `PostToolUse`, and `PostToolUseFailure` have a `tool_use_id`.

**Correlation strategy:** Since `PermissionRequest` lacks a `tool_use_id`, we match it to a lifecycle by:
1. Finding the `PreToolUse` event with the **same `tool_name` and `tool_input`** that occurs just before the `PermissionRequest` timestamp.
2. If multiple candidates, use the closest preceding `PreToolUse` by timestamp.
3. A `PermissionRequest` that cannot be matched to any `PreToolUse` is logged as unmatched (should be rare).

A tool call was "prompted" if it has a matched PermissionRequest event. A tool call was "denied" if it has a PermissionRequest but no PostToolUse (only PostToolUseFailure or nothing).

### Model Types

`ToolEvent`:
- `tool_use_id` must be `Option<String>` (nullable)
- `permission_suggestions` must be `Option<Vec<serde_json::Value>>` (array of objects, not strings)

## Component 2: `clauson tool-events` Subcommand

### Subcommands

#### `tool-events summary` (default)

"Which tools cause the most permission requests?"

```
Tool            Calls   Prompted   Prompt%   Denied   Deny%
────────────────────────────────────────────────────────────
Bash              42       28      66.7%        3     7.1%
Write             18        8      44.4%        0     0.0%
Edit              31        0       0.0%        0     0.0%
Read              56        0       0.0%        0     0.0%
────────────────────────────────────────────────────────────
Total            159       40      25.2%        4     2.5%
```

Columns:
- **Calls**: PreToolUse count per tool
- **Prompted**: PermissionRequest count per tool
- **Prompt%**: Prompted / Calls
- **Denied**: tool_use_ids with PermissionRequest but no PostToolUse
- **Deny%**: Denied / Calls

#### `tool-events list`

Chronological event stream:

```
12:00:00  PreToolUse          Bash     toolu_01ABC
12:00:01  PermissionRequest   Bash     toolu_01ABC
12:00:05  PostToolUse         Bash     toolu_01ABC
```

Flags: `--tool <name>`, `--event <type>`

#### `tool-events timeline`

Per-tool-call lifecycle:

```
toolu_01ABC  Bash  "ls -la"           auto-approved     0.0s
toolu_02DEF  Bash  "cargo test"       prompted->approved  4.2s wait
toolu_03GHI  Bash  "rm -rf /tmp"      prompted->denied    1.1s wait
```

Flag: `--tool <name>`

### Data Loading

1. Take session JSONL path (existing CLI arg)
2. Derive sidecar path: replace `.jsonl` with `.tool-events.jsonl`
3. Parse sidecar with a dedicated parser (separate from session parser)
4. If no sidecar found: print "No tool events file found. Install the clauson-hooks plugin to collect tool event data."

### Implementation in clauson

- New file: `src/cli/tool_events.rs`
- New model types: `ToolEvent`, `ToolEventKind`, `ToolCallLifecycle`
- New parser: `src/parser/tool_events.rs`
- CLI registration in `src/cli/mod.rs`

## Installation

```
/plugin add /path/to/clauson/plugin
```

Or from a git URL once published.
