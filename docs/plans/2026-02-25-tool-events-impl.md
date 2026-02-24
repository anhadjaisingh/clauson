# Tool Events Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a hook plugin that logs tool lifecycle events to a sidecar JSONL file, and a `tool-events` CLI subcommand to analyze that data.

**Architecture:** A Claude Code plugin (`plugin/`) with an async shell hook writes JSONL sidecar files alongside session transcripts. A new `tool-events` command in clauson parses the sidecar and produces summary/list/timeline views. The sidecar parser is separate from the session parser since the data shape is completely different.

**Tech Stack:** Bash (hook script), Rust (clauson), serde/chrono (parsing), clap (CLI), assert_cmd (testing)

---

### Task 1: Create the hook plugin skeleton

**Files:**
- Create: `plugin/plugin.json`
- Create: `plugin/hooks/hooks.json`

**Step 1: Create `plugin/plugin.json`**

```json
{
  "name": "clauson-hooks",
  "description": "Logs tool lifecycle and permission events for analysis with clauson"
}
```

**Step 2: Create `plugin/hooks/hooks.json`**

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/log-tool-event.sh",
            "async": true
          }
        ]
      }
    ],
    "PermissionRequest": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/log-tool-event.sh",
            "async": true
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/log-tool-event.sh",
            "async": true
          }
        ]
      }
    ],
    "PostToolUseFailure": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/log-tool-event.sh",
            "async": true
          }
        ]
      }
    ]
  }
}
```

**Step 3: Verify JSON is valid**

Run: `cat plugin/plugin.json | python3 -m json.tool && cat plugin/hooks/hooks.json | python3 -m json.tool`
Expected: Pretty-printed JSON output with no errors

**Step 4: Commit**

```bash
git add plugin/plugin.json plugin/hooks/hooks.json
git commit -m "feat: add clauson-hooks plugin skeleton with hook definitions"
```

---

### Task 2: Create the hook script

**Files:**
- Create: `plugin/hooks/log-tool-event.sh`

**Step 1: Write `log-tool-event.sh`**

The script reads JSON from stdin, extracts relevant fields, and appends one JSONL line to a sidecar file next to the session transcript.

```bash
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
```

**Step 2: Make script executable**

Run: `chmod +x plugin/hooks/log-tool-event.sh`

**Step 3: Verify script syntax**

Run: `bash -n plugin/hooks/log-tool-event.sh`
Expected: No output (syntax OK)

**Step 4: Test script manually with mock input**

Run:
```bash
echo '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_use_id":"toolu_01","tool_input":{"command":"ls"},"session_id":"test","permission_mode":"default","transcript_path":"/tmp/clauson-test-session.jsonl"}' | bash plugin/hooks/log-tool-event.sh && cat /tmp/clauson-test-session.tool-events.jsonl
```
Expected: One JSON line with event, tool_name, tool_use_id, tool_input, session_id, permission_mode, timestamp

**Step 5: Clean up test file and commit**

```bash
rm -f /tmp/clauson-test-session.tool-events.jsonl
git add plugin/hooks/log-tool-event.sh
git commit -m "feat: add log-tool-event.sh hook script"
```

---

### Task 3: Create test fixture for tool events

**Files:**
- Create: `testdata/test-session.tool-events.jsonl`

We need a test fixture with known data to test the CLI subcommands. Create a file that simulates a session with:
- 5 Bash calls (3 prompted, 1 denied)
- 3 Read calls (0 prompted)
- 2 Write calls (2 prompted, 0 denied)

**Step 1: Create the fixture**

```jsonl
{"event":"PreToolUse","tool_name":"Bash","tool_use_id":"toolu_01","tool_input":{"command":"ls -la"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:00.000Z"}
{"event":"PostToolUse","tool_name":"Bash","tool_use_id":"toolu_01","tool_input":{"command":"ls -la"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:01.000Z"}
{"event":"PreToolUse","tool_name":"Read","tool_use_id":"toolu_02","tool_input":{"file_path":"/tmp/foo.rs"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:02.000Z"}
{"event":"PostToolUse","tool_name":"Read","tool_use_id":"toolu_02","tool_input":{"file_path":"/tmp/foo.rs"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:02.500Z"}
{"event":"PreToolUse","tool_name":"Bash","tool_use_id":"toolu_03","tool_input":{"command":"cargo test"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:03.000Z"}
{"event":"PermissionRequest","tool_name":"Bash","tool_use_id":"toolu_03","tool_input":{"command":"cargo test"},"session_id":"test-sess","permission_mode":"default","permission_suggestions":["allow_bash_command:cargo*"],"timestamp":"2026-02-24T12:00:03.100Z"}
{"event":"PostToolUse","tool_name":"Bash","tool_use_id":"toolu_03","tool_input":{"command":"cargo test"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:07.000Z"}
{"event":"PreToolUse","tool_name":"Write","tool_use_id":"toolu_04","tool_input":{"file_path":"/tmp/out.rs","content":"fn main() {}"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:08.000Z"}
{"event":"PermissionRequest","tool_name":"Write","tool_use_id":"toolu_04","tool_input":{"file_path":"/tmp/out.rs","content":"fn main() {}"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:08.200Z"}
{"event":"PostToolUse","tool_name":"Write","tool_use_id":"toolu_04","tool_input":{"file_path":"/tmp/out.rs","content":"fn main() {}"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:09.000Z"}
{"event":"PreToolUse","tool_name":"Read","tool_use_id":"toolu_05","tool_input":{"file_path":"/tmp/bar.rs"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:10.000Z"}
{"event":"PostToolUse","tool_name":"Read","tool_use_id":"toolu_05","tool_input":{"file_path":"/tmp/bar.rs"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:10.300Z"}
{"event":"PreToolUse","tool_name":"Bash","tool_use_id":"toolu_06","tool_input":{"command":"rm -rf /tmp/stuff"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:11.000Z"}
{"event":"PermissionRequest","tool_name":"Bash","tool_use_id":"toolu_06","tool_input":{"command":"rm -rf /tmp/stuff"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:11.100Z"}
{"event":"PostToolUseFailure","tool_name":"Bash","tool_use_id":"toolu_06","tool_input":{"command":"rm -rf /tmp/stuff"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:12.000Z"}
{"event":"PreToolUse","tool_name":"Bash","tool_use_id":"toolu_07","tool_input":{"command":"git status"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:13.000Z"}
{"event":"PermissionRequest","tool_name":"Bash","tool_use_id":"toolu_07","tool_input":{"command":"git status"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:13.050Z"}
{"event":"PostToolUse","tool_name":"Bash","tool_use_id":"toolu_07","tool_input":{"command":"git status"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:14.000Z"}
{"event":"PreToolUse","tool_name":"Bash","tool_use_id":"toolu_08","tool_input":{"command":"echo hello"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:15.000Z"}
{"event":"PostToolUse","tool_name":"Bash","tool_use_id":"toolu_08","tool_input":{"command":"echo hello"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:15.500Z"}
{"event":"PreToolUse","tool_name":"Read","tool_use_id":"toolu_09","tool_input":{"file_path":"/tmp/baz.rs"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:16.000Z"}
{"event":"PostToolUse","tool_name":"Read","tool_use_id":"toolu_09","tool_input":{"file_path":"/tmp/baz.rs"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:16.200Z"}
{"event":"PreToolUse","tool_name":"Write","tool_use_id":"toolu_10","tool_input":{"file_path":"/tmp/new.rs","content":"// new"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:17.000Z"}
{"event":"PermissionRequest","tool_name":"Write","tool_use_id":"toolu_10","tool_input":{"file_path":"/tmp/new.rs","content":"// new"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:17.100Z"}
{"event":"PostToolUse","tool_name":"Write","tool_use_id":"toolu_10","tool_input":{"file_path":"/tmp/new.rs","content":"// new"},"session_id":"test-sess","permission_mode":"default","timestamp":"2026-02-24T12:00:18.000Z"}
```

This gives us:
- Bash: 5 calls, 3 prompted (toolu_03, toolu_06, toolu_07), 1 denied (toolu_06)
- Read: 3 calls, 0 prompted, 0 denied
- Write: 2 calls, 2 prompted (toolu_04, toolu_10), 0 denied
- **Total: 10 calls, 5 prompted, 1 denied**

**Step 2: Verify fixture is valid JSONL**

Run: `while IFS= read -r line; do echo "$line" | python3 -m json.tool > /dev/null || echo "INVALID: $line"; done < testdata/test-session.tool-events.jsonl && echo "All lines valid"`
Expected: "All lines valid"

**Step 3: Commit**

```bash
git add testdata/test-session.tool-events.jsonl
git commit -m "test: add tool-events JSONL test fixture"
```

---

### Task 4: Add tool event model types

**Files:**
- Create: `src/model/tool_event.rs`
- Modify: `src/model/mod.rs:1-2` (add module)

**Step 1: Write the types**

Create `src/model/tool_event.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct ToolEvent {
    pub event: ToolEventKind,
    pub tool_name: String,
    pub tool_use_id: String,
    #[serde(default)]
    pub tool_input: serde_json::Value,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub permission_mode: Option<String>,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub permission_suggestions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum ToolEventKind {
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    PostToolUseFailure,
}

impl std::fmt::Display for ToolEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolEventKind::PreToolUse => write!(f, "PreToolUse"),
            ToolEventKind::PermissionRequest => write!(f, "PermissionRequest"),
            ToolEventKind::PostToolUse => write!(f, "PostToolUse"),
            ToolEventKind::PostToolUseFailure => write!(f, "PostToolUseFailure"),
        }
    }
}

/// Reconstructed lifecycle for a single tool call (grouped by tool_use_id).
#[derive(Debug)]
pub struct ToolCallLifecycle {
    pub tool_use_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub pre_tool_use: Option<DateTime<Utc>>,
    pub permission_request: Option<DateTime<Utc>>,
    pub completion: Option<DateTime<Utc>>,
    pub succeeded: bool,
}

impl ToolCallLifecycle {
    pub fn was_prompted(&self) -> bool {
        self.permission_request.is_some()
    }

    pub fn was_denied(&self) -> bool {
        self.permission_request.is_some() && !self.succeeded
    }

    /// Time between PermissionRequest and completion, in seconds.
    pub fn permission_wait_secs(&self) -> Option<f64> {
        let perm = self.permission_request?;
        let end = self.completion?;
        Some((end - perm).num_milliseconds() as f64 / 1000.0)
    }

    pub fn status_label(&self) -> &str {
        if !self.was_prompted() {
            "auto-approved"
        } else if self.succeeded {
            "prompted->approved"
        } else {
            "prompted->denied"
        }
    }
}

/// Group events by tool_use_id into lifecycles.
pub fn build_lifecycles(events: &[ToolEvent]) -> Vec<ToolCallLifecycle> {
    let mut map: HashMap<String, ToolCallLifecycle> = HashMap::new();
    // Track insertion order
    let mut order: Vec<String> = Vec::new();

    for event in events {
        let entry = map.entry(event.tool_use_id.clone()).or_insert_with(|| {
            order.push(event.tool_use_id.clone());
            ToolCallLifecycle {
                tool_use_id: event.tool_use_id.clone(),
                tool_name: event.tool_name.clone(),
                tool_input: event.tool_input.clone(),
                pre_tool_use: None,
                permission_request: None,
                completion: None,
                succeeded: false,
            }
        });

        match event.event {
            ToolEventKind::PreToolUse => entry.pre_tool_use = Some(event.timestamp),
            ToolEventKind::PermissionRequest => entry.permission_request = Some(event.timestamp),
            ToolEventKind::PostToolUse => {
                entry.completion = Some(event.timestamp);
                entry.succeeded = true;
            }
            ToolEventKind::PostToolUseFailure => {
                entry.completion = Some(event.timestamp);
                entry.succeeded = false;
            }
        }
    }

    order.into_iter().filter_map(|id| map.remove(&id)).collect()
}
```

**Step 2: Register the module in `src/model/mod.rs`**

Change `src/model/mod.rs` from:
```rust
pub mod block;
pub mod session;
pub mod turn;
pub mod types;
```
to:
```rust
pub mod block;
pub mod session;
pub mod tool_event;
pub mod turn;
pub mod types;
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors

**Step 4: Commit**

```bash
git add src/model/tool_event.rs src/model/mod.rs
git commit -m "feat: add ToolEvent, ToolEventKind, and ToolCallLifecycle model types"
```

---

### Task 5: Add tool events parser

**Files:**
- Create: `src/parser/tool_events.rs`
- Modify: `src/parser/mod.rs:1-2` (add module)

**Step 1: Write the parser**

Create `src/parser/tool_events.rs`:

```rust
use std::io::BufRead;
use std::path::Path;

use crate::model::tool_event::ToolEvent;

/// Parse a tool-events JSONL sidecar file into a Vec<ToolEvent>.
pub fn parse_tool_events(path: &Path) -> anyhow::Result<Vec<ToolEvent>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<ToolEvent>(&line) {
            events.push(event);
        }
    }

    Ok(events)
}

/// Derive the sidecar path from a session JSONL path.
/// e.g., "foo.jsonl" -> "foo.tool-events.jsonl"
pub fn sidecar_path(session_path: &Path) -> std::path::PathBuf {
    let stem = session_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let parent = session_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{stem}.tool-events.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sidecar_path_replaces_extension() {
        let p = PathBuf::from("/home/user/.claude/sessions/abc.jsonl");
        assert_eq!(
            sidecar_path(&p),
            PathBuf::from("/home/user/.claude/sessions/abc.tool-events.jsonl")
        );
    }

    #[test]
    fn sidecar_path_no_extension() {
        let p = PathBuf::from("/tmp/session");
        assert_eq!(
            sidecar_path(&p),
            PathBuf::from("/tmp/session.tool-events.jsonl")
        );
    }

    #[test]
    fn parse_fixture() {
        let p = PathBuf::from("testdata/test-session.tool-events.jsonl");
        if !p.exists() {
            return; // skip if fixture not yet created
        }
        let events = parse_tool_events(&p).unwrap();
        assert!(!events.is_empty());
        // We know the fixture has 10 tool calls across PreToolUse/PostToolUse/PermissionRequest
        // Check that we can parse all lines
        assert!(events.len() > 10);
    }
}
```

**Step 2: Register module in `src/parser/mod.rs`**

Add `pub mod tool_events;` to `src/parser/mod.rs` after existing modules.

**Step 3: Run tests**

Run: `cargo test parser::tool_events`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/parser/tool_events.rs src/parser/mod.rs
git commit -m "feat: add tool events JSONL parser with sidecar path derivation"
```

---

### Task 6: Add tool-events CLI subcommand with summary

**Files:**
- Create: `src/cli/tool_events.rs`
- Modify: `src/cli/mod.rs:1-50` (add module and Command variant)
- Modify: `src/main.rs:1-25` (add match arm)

**Step 1: Write the CLI module**

Create `src/cli/tool_events.rs`:

```rust
use anyhow::Result;
use clap::Subcommand;
use std::collections::HashMap;
use std::path::Path;

use clauson::model::tool_event::{build_lifecycles, ToolCallLifecycle, ToolEventKind};
use clauson::parser::tool_events::{parse_tool_events, sidecar_path};

use super::output;

#[derive(Subcommand)]
pub enum ToolEventsAction {
    /// Aggregated permission stats per tool (default)
    Summary,
    /// Chronological event stream
    List {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
        /// Filter by event type
        #[arg(long)]
        event: Option<String>,
    },
    /// Per-tool-call lifecycle with permission wait times
    Timeline {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
    },
}

pub fn run(session_path: &Path, action: Option<&ToolEventsAction>, json: bool) -> Result<()> {
    let sidecar = sidecar_path(session_path);
    if !sidecar.exists() {
        eprintln!(
            "No tool events file found at: {}\n\
             Install the clauson-hooks plugin to collect tool event data:\n\
             /plugin add <path-to-clauson>/plugin",
            sidecar.display()
        );
        std::process::exit(1);
    }

    let events = parse_tool_events(&sidecar)?;
    if events.is_empty() {
        println!("No tool events recorded.");
        return Ok(());
    }

    match action {
        None | Some(ToolEventsAction::Summary) => run_summary(&events, json),
        Some(ToolEventsAction::List { tool, event }) => {
            run_list(&events, tool.as_deref(), event.as_deref(), json)
        }
        Some(ToolEventsAction::Timeline { tool }) => {
            let lifecycles = build_lifecycles(&events);
            run_timeline(&lifecycles, tool.as_deref(), json)
        }
    }
}

fn run_summary(events: &[clauson::model::tool_event::ToolEvent], json: bool) -> Result<()> {
    let lifecycles = build_lifecycles(events);

    struct ToolStats {
        calls: usize,
        prompted: usize,
        denied: usize,
    }

    let mut stats: HashMap<String, ToolStats> = HashMap::new();
    for lc in &lifecycles {
        let entry = stats.entry(lc.tool_name.clone()).or_insert(ToolStats {
            calls: 0,
            prompted: 0,
            denied: 0,
        });
        entry.calls += 1;
        if lc.was_prompted() {
            entry.prompted += 1;
        }
        if lc.was_denied() {
            entry.denied += 1;
        }
    }

    let mut entries: Vec<_> = stats.into_iter().collect();
    entries.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));

    let total_calls: usize = entries.iter().map(|(_, s)| s.calls).sum();
    let total_prompted: usize = entries.iter().map(|(_, s)| s.prompted).sum();
    let total_denied: usize = entries.iter().map(|(_, s)| s.denied).sum();

    if json {
        let mut json_entries: Vec<_> = entries
            .iter()
            .map(|(name, s)| {
                serde_json::json!({
                    "tool_name": name,
                    "calls": s.calls,
                    "prompted": s.prompted,
                    "prompt_percent": format!("{:.1}", pct(s.prompted, s.calls)),
                    "denied": s.denied,
                    "deny_percent": format!("{:.1}", pct(s.denied, s.calls)),
                })
            })
            .collect();
        json_entries.push(serde_json::json!({
            "tool_name": "Total",
            "calls": total_calls,
            "prompted": total_prompted,
            "prompt_percent": format!("{:.1}", pct(total_prompted, total_calls)),
            "denied": total_denied,
            "deny_percent": format!("{:.1}", pct(total_denied, total_calls)),
        }));
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Tool"),
            output::Column::right("Calls"),
            output::Column::right("Prompted"),
            output::Column::right("Prompt%"),
            output::Column::right("Denied"),
            output::Column::right("Deny%"),
        ]);
        for (name, s) in &entries {
            table.add_row(vec![
                name.clone(),
                s.calls.to_string(),
                s.prompted.to_string(),
                format!("{:.1}%", pct(s.prompted, s.calls)),
                s.denied.to_string(),
                format!("{:.1}%", pct(s.denied, s.calls)),
            ]);
        }
        table.print_with_total(&format!(
            "Total: {} calls, {} prompted ({:.1}%), {} denied ({:.1}%)",
            total_calls,
            total_prompted,
            pct(total_prompted, total_calls),
            total_denied,
            pct(total_denied, total_calls),
        ));
    }

    Ok(())
}

fn run_list(
    events: &[clauson::model::tool_event::ToolEvent],
    tool_filter: Option<&str>,
    event_filter: Option<&str>,
    json: bool,
) -> Result<()> {
    let filtered: Vec<_> = events
        .iter()
        .filter(|e| {
            if let Some(tool) = tool_filter {
                if e.tool_name != tool {
                    return false;
                }
            }
            if let Some(evt) = event_filter {
                if e.event.to_string() != evt {
                    return false;
                }
            }
            true
        })
        .collect();

    if json {
        let json_entries: Vec<_> = filtered
            .iter()
            .map(|e| {
                serde_json::json!({
                    "timestamp": e.timestamp.format("%H:%M:%S%.3f").to_string(),
                    "event": e.event.to_string(),
                    "tool_name": e.tool_name,
                    "tool_use_id": e.tool_use_id,
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Time"),
            output::Column::left("Event"),
            output::Column::left("Tool"),
            output::Column::left("Tool Use ID"),
        ]);
        for e in &filtered {
            table.add_row(vec![
                e.timestamp.format("%H:%M:%S").to_string(),
                e.event.to_string(),
                e.tool_name.clone(),
                output::truncate(&e.tool_use_id, 20),
            ]);
        }
        table.print();
    }

    Ok(())
}

fn run_timeline(
    lifecycles: &[ToolCallLifecycle],
    tool_filter: Option<&str>,
    json: bool,
) -> Result<()> {
    let filtered: Vec<_> = lifecycles
        .iter()
        .filter(|lc| {
            if let Some(tool) = tool_filter {
                lc.tool_name == tool
            } else {
                true
            }
        })
        .collect();

    if json {
        let json_entries: Vec<_> = filtered
            .iter()
            .map(|lc| {
                let detail = extract_tool_detail(&lc.tool_name, &lc.tool_input);
                serde_json::json!({
                    "tool_use_id": lc.tool_use_id,
                    "tool_name": lc.tool_name,
                    "detail": detail,
                    "status": lc.status_label(),
                    "wait_secs": lc.permission_wait_secs(),
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Tool Use ID"),
            output::Column::left("Tool"),
            output::Column::left("Detail"),
            output::Column::left("Status"),
            output::Column::right("Wait"),
        ]);
        for lc in &filtered {
            let detail = extract_tool_detail(&lc.tool_name, &lc.tool_input);
            let wait = lc
                .permission_wait_secs()
                .map(|s| format!("{s:.1}s"))
                .unwrap_or_default();
            table.add_row(vec![
                output::truncate(&lc.tool_use_id, 20),
                lc.tool_name.clone(),
                detail,
                lc.status_label().to_string(),
                wait,
            ]);
        }
        table.print();
    }

    Ok(())
}

fn extract_tool_detail(tool_name: &str, input: &serde_json::Value) -> String {
    let raw = match tool_name {
        "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("(no command)"),
        "Read" | "Write" | "Edit" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("(no path)"),
        "Grep" | "Glob" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("(no pattern)"),
        _ => {
            let s = input.to_string();
            return output::truncate(&s, 40);
        }
    };
    output::truncate(raw, 40)
}

fn pct(part: usize, total: usize) -> f64 {
    if total > 0 {
        (part as f64 / total as f64) * 100.0
    } else {
        0.0
    }
}
```

**Step 2: Register module in `src/cli/mod.rs`**

Add `pub mod tool_events;` to the module list and add the `ToolEvents` variant to the `Command` enum:

In `src/cli/mod.rs`, add to modules (after `pub mod turns;`):
```rust
pub mod tool_events;
```

Add to the `Command` enum (after `Stats` variant):
```rust
    /// Analyze tool lifecycle and permission events from sidecar log
    #[command(name = "tool-events")]
    ToolEvents {
        #[command(subcommand)]
        action: Option<tool_events::ToolEventsAction>,
    },
```

**Step 3: Add match arm in `src/main.rs`**

Add after the `Stats` match arm:
```rust
        cli::Command::ToolEvents { action } => {
            cli::tool_events::run(&cli.file, action.as_ref(), cli.json)?;
        }
```

Note: `tool-events` does NOT need the parsed `session` — it reads its own sidecar file. The session is parsed unconditionally in `main.rs` currently, but that's acceptable overhead. If desired, we can optimize later by only parsing the session when needed.

**Step 4: Verify it compiles**

Run: `cargo clippy -- -D warnings`
Expected: No errors or warnings

**Step 5: Commit**

```bash
git add src/cli/tool_events.rs src/cli/mod.rs src/main.rs
git commit -m "feat: add tool-events subcommand with summary, list, and timeline"
```

---

### Task 7: Add integration tests for tool-events

**Files:**
- Modify: `tests/cli_tests.rs` (append new tests)

**Step 1: Write tests**

Append to `tests/cli_tests.rs`:

```rust
// === tool-events ===

const TOOL_EVENTS_SESSION: &str = "testdata/test-session.jsonl";

// We need a session JSONL file whose sidecar exists. The fixture
// testdata/test-session.tool-events.jsonl is the sidecar for
// testdata/test-session.jsonl.

#[test]
fn tool_events_missing_sidecar() {
    // MEDIUM_FILE has no sidecar, so this should fail with helpful message
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tool-events"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No tool events file found"));
}

#[test]
fn tool_events_summary_default() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tool"))
        .stdout(predicate::str::contains("Calls"))
        .stdout(predicate::str::contains("Prompted"))
        .stdout(predicate::str::contains("Denied"));
}

#[test]
fn tool_events_summary_json() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "summary", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn tool_events_summary_counts_correct() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "summary", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    // Last entry is Total
    let total = parsed.last().unwrap();
    assert_eq!(total["calls"], 10);
    assert_eq!(total["prompted"], 5);
    assert_eq!(total["denied"], 1);
}

#[test]
fn tool_events_list() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PreToolUse"))
        .stdout(predicate::str::contains("Bash"));
}

#[test]
fn tool_events_list_filter_tool() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "list",
            "--tool",
            "Read",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for entry in &parsed {
        assert_eq!(entry["tool_name"], "Read");
    }
}

#[test]
fn tool_events_list_filter_event() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "list",
            "--event",
            "PermissionRequest",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for entry in &parsed {
        assert_eq!(entry["event"], "PermissionRequest");
    }
    assert_eq!(parsed.len(), 5); // 3 Bash + 2 Write
}

#[test]
fn tool_events_timeline() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "timeline"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto-approved"))
        .stdout(predicate::str::contains("prompted->approved"));
}

#[test]
fn tool_events_timeline_filter() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "timeline",
            "--tool",
            "Bash",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed.len(), 5); // 5 Bash calls
    for entry in &parsed {
        assert_eq!(entry["tool_name"], "Bash");
    }
}

#[test]
fn tool_events_timeline_shows_denied() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "timeline",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    let denied: Vec<_> = parsed.iter().filter(|e| e["status"] == "prompted->denied").collect();
    assert_eq!(denied.len(), 1);
}

#[test]
fn tool_events_default_is_summary() {
    let with = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "summary", "--json"])
        .output()
        .unwrap();
    let without = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "--json"])
        .output()
        .unwrap();
    assert_eq!(with.stdout, without.stdout);
}
```

**Step 2: Create a minimal session JSONL for the test fixture**

We need `testdata/test-session.jsonl` to exist (it's the session file; the tool-events sidecar is derived from it). It just needs to be a valid session file that parses without error:

```jsonl
{"type":"user","uuid":"u1","parentUuid":null,"timestamp":"2026-02-24T12:00:00.000Z","sessionId":"test-sess","isSidechain":false,"isMeta":false,"message":{"role":"user","content":"Test session for tool events"}}
{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-02-24T12:00:01.000Z","sessionId":"test-sess","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"OK."}],"stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":5}}}
```

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass (existing + new)

**Step 4: Commit**

```bash
git add tests/cli_tests.rs testdata/test-session.jsonl
git commit -m "test: add integration tests for tool-events subcommand"
```

---

### Task 8: Run full validation

**Step 1: Lint**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 2: Full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 3: Manual smoke test**

Run: `cargo run -- testdata/test-session.jsonl tool-events`
Expected: Summary table showing Bash/Read/Write with correct counts

Run: `cargo run -- testdata/test-session.jsonl tool-events timeline`
Expected: Timeline showing auto-approved, prompted->approved, and prompted->denied entries

**Step 4: Final commit if any fixes were needed**
