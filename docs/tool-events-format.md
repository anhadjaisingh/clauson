# Tool Events Sidecar Format

Claude Code hooks emit tool event data as JSONL sidecar files alongside session recordings. This document describes the real event structure based on production data.

## Sidecar File Naming

For a session file `<uuid>.jsonl`, the sidecar is `<uuid>.tool-events.jsonl`. Each line is a JSON object representing one hook invocation.

## Event Types

### PreToolUse

Fires before tool execution begins. Always has a `tool_use_id`.

| Field | Type | Notes |
|-------|------|-------|
| `event` | `"PreToolUse"` | |
| `tool_name` | `string` | e.g. `"Bash"`, `"Read"`, `"Write"` |
| `tool_use_id` | `string` | Unique ID for this tool call |
| `tool_input` | `object` | Tool-specific input parameters |
| `session_id` | `string` | Session UUID |
| `permission_mode` | `string` | e.g. `"default"` |
| `timestamp` | `string` | ISO 8601 UTC timestamp |

### PermissionRequest

Fires when the user is prompted to approve/deny a tool call. **`tool_use_id` is `null`** in real data — this is the key difference from other event types.

| Field | Type | Notes |
|-------|------|-------|
| `event` | `"PermissionRequest"` | |
| `tool_name` | `string` | |
| `tool_use_id` | `null` | Always null in production data |
| `tool_input` | `object` | Same input as the corresponding PreToolUse |
| `session_id` | `string` | |
| `permission_mode` | `string` | |
| `timestamp` | `string` | ISO 8601 UTC timestamp |
| `permission_suggestions` | `array \| null` | Array of suggestion objects, or null |

#### `permission_suggestions` Object Schema

Each element in `permission_suggestions` is an object (not a string):

```json
{
  "type": "addRules",
  "rules": ["Bash(npm run build)", "Bash(npm test)"],
  "behavior": "allow",
  "destination": "session"
}
```

Or for mode-setting suggestions:

```json
{
  "type": "setMode",
  "mode": "autoEdit"
}
```

Fields:
- `type`: `"addRules"` or `"setMode"`
- `rules` (addRules only): Array of rule strings
- `behavior` (addRules only): `"allow"` or `"deny"`
- `destination` (addRules only): `"session"` or `"global"`
- `mode` (setMode only): The permission mode to set

### PostToolUse

Fires after successful tool execution. Always has a `tool_use_id`.

| Field | Type | Notes |
|-------|------|-------|
| `event` | `"PostToolUse"` | |
| `tool_name` | `string` | |
| `tool_use_id` | `string` | Same ID as the PreToolUse |
| `tool_input` | `object` | |
| `session_id` | `string` | |
| `permission_mode` | `string` | |
| `timestamp` | `string` | |

### PostToolUseFailure

Fires after tool execution fails (including when user denies permission).

Same fields as PostToolUse, with `event` = `"PostToolUseFailure"`.

## Lifecycle Correlation

Events for a single tool call form a lifecycle:

1. **Auto-approved**: `PreToolUse` → `PostToolUse`
2. **Prompted → approved**: `PreToolUse` → `PermissionRequest` → `PostToolUse`
3. **Prompted → denied**: `PreToolUse` → `PermissionRequest` → `PostToolUseFailure`

Since `PermissionRequest` has `tool_use_id: null`, correlation cannot use `tool_use_id` alone. Instead:
- `PreToolUse`, `PostToolUse`, and `PostToolUseFailure` are grouped by `tool_use_id`
- `PermissionRequest` is matched to the most recent `PreToolUse` with the same `tool_name` AND `tool_input`

This works because:
- The `PermissionRequest` hook always fires after the `PreToolUse` hook for the same tool call
- The `tool_input` is identical between both events
- Events are ordered chronologically in the sidecar file

## Real JSON Examples

### Auto-approved Bash

```json
{"event":"PreToolUse","tool_name":"Bash","tool_use_id":"toolu_abc123","tool_input":{"command":"ls -la"},"session_id":"sess-1","permission_mode":"default","timestamp":"2026-02-24T12:00:00.000Z"}
{"event":"PostToolUse","tool_name":"Bash","tool_use_id":"toolu_abc123","tool_input":{"command":"ls -la"},"session_id":"sess-1","permission_mode":"default","timestamp":"2026-02-24T12:00:01.000Z"}
```

### Prompted → Approved with permission_suggestions

```json
{"event":"PreToolUse","tool_name":"Bash","tool_use_id":"toolu_def456","tool_input":{"command":"cargo test"},"session_id":"sess-1","permission_mode":"default","timestamp":"2026-02-24T12:00:02.000Z"}
{"event":"PermissionRequest","tool_name":"Bash","tool_use_id":null,"tool_input":{"command":"cargo test"},"session_id":"sess-1","permission_mode":"default","permission_suggestions":[{"type":"addRules","rules":["Bash(cargo test)"],"behavior":"allow","destination":"session"}],"timestamp":"2026-02-24T12:00:02.100Z"}
{"event":"PostToolUse","tool_name":"Bash","tool_use_id":"toolu_def456","tool_input":{"command":"cargo test"},"session_id":"sess-1","permission_mode":"default","timestamp":"2026-02-24T12:00:05.000Z"}
```

### Prompted → Denied

```json
{"event":"PreToolUse","tool_name":"WebFetch","tool_use_id":"toolu_ghi789","tool_input":{"url":"https://example.com"},"session_id":"sess-1","permission_mode":"default","timestamp":"2026-02-24T12:00:06.000Z"}
{"event":"PermissionRequest","tool_name":"WebFetch","tool_use_id":null,"tool_input":{"url":"https://example.com"},"session_id":"sess-1","permission_mode":"default","permission_suggestions":[{"type":"addRules","rules":["WebFetch(https://example.com)"],"behavior":"allow","destination":"session"}],"timestamp":"2026-02-24T12:00:06.100Z"}
{"event":"PostToolUseFailure","tool_name":"WebFetch","tool_use_id":"toolu_ghi789","tool_input":{"url":"https://example.com"},"session_id":"sess-1","permission_mode":"default","timestamp":"2026-02-24T12:00:08.000Z"}
```
