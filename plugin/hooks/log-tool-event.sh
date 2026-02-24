#!/usr/bin/env bash
set -euo pipefail

# Read full JSON from stdin
input=$(cat)

# Extract transcript path to derive sidecar location
transcript_path=$(printf '%s' "$input" | jq -r '.transcript_path // empty')
if [ -z "$transcript_path" ]; then
  exit 0
fi

# Derive sidecar path: session.jsonl -> session.tool-events.jsonl
sidecar="${transcript_path%.jsonl}.tool-events.jsonl"

# Build output JSON with common fields + event-specific fields
printf '%s' "$input" | jq -c '{
  event: .hook_event_name,
  tool_name: .tool_name,
  tool_use_id: .tool_use_id,
  tool_input: .tool_input,
  session_id: .session_id,
  permission_mode: .permission_mode,
  timestamp: (now | strftime("%Y-%m-%dT%H:%M:%S.000Z"))
} + (if .hook_event_name == "PermissionRequest" then {permission_suggestions: .permission_suggestions} else {} end)' >> "$sidecar"
