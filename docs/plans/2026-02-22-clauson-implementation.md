# Clauson Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust CLI that parses Claude Code session JSONL into a typed DAG and exposes querying via noun-verb subcommands.

**Architecture:** Two-layer parser (raw JSONL -> semantic blocks), arena-based DAG with secondary indexes, thin CLI handlers over a library core.

**Tech Stack:** Rust, clap (CLI), serde/serde_json (parsing), chrono (timestamps), tabled (table output), assert_cmd (e2e tests)

**Design doc:** `docs/plans/2026-02-22-clauson-design.md`

---

### Task 1: Project Scaffolding + Shared Types

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/model/mod.rs`
- Create: `src/model/types.rs`
- Create: `src/parser/mod.rs`
- Create: `src/cli/mod.rs`

**Step 1: Initialize the Rust project**

Run: `cargo init --name clauson` in project root.

This creates `Cargo.toml` and `src/main.rs`. Then update `Cargo.toml`:

```toml
[package]
name = "clauson"
version = "0.1.0"
edition = "2024"

[dependencies]
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tabled = "0.17"
anyhow = "1"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

**Step 2: Create module structure**

Create `src/lib.rs`:
```rust
pub mod model;
pub mod parser;
```

Create `src/model/mod.rs`:
```rust
pub mod types;
```

Create `src/parser/mod.rs`:
```rust
// Parser module - to be implemented
```

Create `src/cli/mod.rs`:
```rust
// CLI module - to be implemented
```

Update `src/main.rs`:
```rust
fn main() {
    println!("clauson - Claude session JSONL analyzer");
}
```

**Step 3: Write test for shared types**

Add to `src/model/types.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    User,
    Assistant,
    Tool,
    System,
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockType::User => write!(f, "user"),
            BlockType::Assistant => write!(f, "assistant"),
            BlockType::Tool => write!(f, "tool"),
            BlockType::System => write!(f, "system"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl TokenUsage {
    pub fn total_input(&self) -> u64 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }

    pub fn total(&self) -> u64 {
        self.total_input() + self.output_tokens
    }

    pub fn merge(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_input_tokens += other.cache_creation_input_tokens;
        self.cache_read_input_tokens += other.cache_read_input_tokens;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntryMetadata {
    pub version: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub slug: Option<String>,
    pub is_sidechain: bool,
}

/// Reference to a raw JSONL line for provenance tracking
#[derive(Debug, Clone, Serialize)]
pub struct RawLineRef {
    pub line_number: usize,   // 1-indexed
    pub byte_offset: usize,
    pub byte_length: usize,
}

pub type NodeId = usize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_usage_total_input() {
        let t = TokenUsage {
            input_tokens: 10,
            output_tokens: 5,
            cache_creation_input_tokens: 100,
            cache_read_input_tokens: 200,
        };
        assert_eq!(t.total_input(), 310);
        assert_eq!(t.total(), 315);
    }

    #[test]
    fn token_usage_merge() {
        let mut a = TokenUsage { input_tokens: 10, output_tokens: 5, ..Default::default() };
        let b = TokenUsage { input_tokens: 20, output_tokens: 15, ..Default::default() };
        a.merge(&b);
        assert_eq!(a.input_tokens, 30);
        assert_eq!(a.output_tokens, 20);
    }

    #[test]
    fn block_type_display() {
        assert_eq!(BlockType::User.to_string(), "user");
        assert_eq!(BlockType::Assistant.to_string(), "assistant");
        assert_eq!(BlockType::Tool.to_string(), "tool");
        assert_eq!(BlockType::System.to_string(), "system");
    }
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: PASS (3 tests)

**Step 5: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: project scaffolding with shared types"
```

---

### Task 2: Block Model + BlockInfo Trait

**Files:**
- Create: `src/model/block.rs`
- Modify: `src/model/mod.rs`

**Step 1: Write the failing test**

Create `src/model/block.rs` with block types and tests. Key structures:

- `Block` enum with `User(UserBlock)`, `Assistant(AssistantBlock)`, `Tool(ToolBlock)`, `System(SystemBlock)`
- `UserBlock`: uuid, parent_uuid, timestamp, session_id, content (Option<String>), is_meta (bool), metadata
- `AssistantBlock`: uuid, parent_uuid, timestamp, session_id, request_id, model, content, thinking, tool_calls (Vec<ToolCall>), stop_reason, tokens, metadata
- `ToolCall`: tool_use_id, tool_name, input (serde_json::Value)
- `ToolBlock`: tool_use_id, tool_name, input, output, is_error, assistant_uuid, result_uuid, timestamp, metadata
- `SystemBlock`: uuid, parent_uuid, timestamp, session_id, subtype (SystemSubtype enum), metadata
- `SystemSubtype`: TurnDuration { duration_ms: u64 }, StopHookSummary, CompactBoundary { trigger: String, pre_tokens: u64 }
- `BlockInfo` trait: block_type(), timestamp(), uuid(), parent_uuid(), tokens(), duration_ms()
- Implement `BlockInfo` for `Block` by delegating to variants

Tests:
- `block_info_user` - create UserBlock, verify block_type() == User, tokens() == None
- `block_info_assistant` - create AssistantBlock with tokens, verify block_type() == Assistant, tokens() == Some(...)
- `block_info_tool` - create ToolBlock, verify block_type() == Tool
- `block_info_system_turn_duration` - create SystemBlock with TurnDuration, verify duration_ms() == Some(...)

**Step 2: Run tests to verify they pass**

Run: `cargo test model::block`
Expected: PASS

**Step 3: Commit**

```bash
git add src/model/
git commit -m "feat: block model types with BlockInfo trait"
```

---

### Task 3: Raw JSONL Deserialization

**Files:**
- Create: `src/parser/raw.rs`
- Modify: `src/parser/mod.rs`

This is the most complex parsing task. The raw layer maps directly to the JSONL format.

**Step 1: Write failing tests for raw deserialization**

Tests to write in `src/parser/raw.rs` (in `#[cfg(test)]` module):

1. `parse_user_text_message` - deserialize a user entry with string content
2. `parse_user_tool_result` - deserialize a user entry with tool_result array content
3. `parse_assistant_text` - deserialize assistant entry with text content block
4. `parse_assistant_tool_use` - deserialize assistant entry with tool_use content block
5. `parse_assistant_thinking` - deserialize assistant entry with thinking content block
6. `parse_system_turn_duration` - deserialize system entry with subtype "turn_duration"
7. `parse_system_compact_boundary` - deserialize system entry with subtype "compact_boundary"
8. `parse_progress_skipped` - verify progress entries parse into the Progress variant
9. `parse_file_history_snapshot` - verify these parse into FileHistorySnapshot variant
10. `parse_unknown_type` - verify unknown types don't panic (use `#[serde(other)]`)

Each test will use a raw JSON string based on the actual test data format.

**Step 2: Implement raw deserialization structs**

Key design decisions for serde:
- Use `#[serde(tag = "type")]` on `RawEntry` enum to dispatch by the `type` field
- Use `#[serde(rename_all = "camelCase")]` on structs for camelCase JSON field names
- `message.content` uses `#[serde(untagged)]` enum: `MessageContent::Text(String)` | `MessageContent::Blocks(Vec<ContentBlock>)`
- Assistant `message.content` blocks use `#[serde(tag = "type")]` enum: `ContentBlock::Text { text }` | `ContentBlock::ToolUse { id, name, input }` | `ContentBlock::Thinking { thinking }`
- `message.usage` maps to a `RawUsage` struct with all 4 token fields
- Unknown content block types captured via `#[serde(other)]` Unknown variant

Structs needed:
```
RawEntry (tagged enum on "type")
├── User(RawUserEntry) - rename "user"
│   ├── common fields: uuid, parent_uuid, timestamp, session_id, version, cwd, git_branch, is_sidechain, slug
│   ├── message: RawMessage
│   ├── is_meta: Option<bool>
│   ├── tool_use_result: Option<serde_json::Value>
│   └── source_tool_assistant_uuid: Option<String> (rename "sourceToolAssistantUUID")
├── Assistant(RawAssistantEntry) - rename "assistant"
│   ├── common fields
│   ├── message: RawAssistantMessage
│   └── request_id: String
├── System(RawSystemEntry) - rename "system"
│   ├── common fields
│   ├── subtype: String
│   ├── duration_ms: Option<u64>
│   ├── compact_metadata: Option<RawCompactMetadata>
│   └── (other system-specific fields as serde_json::Value via flatten)
├── Progress(RawProgressEntry) - rename "progress"
├── FileHistorySnapshot(serde_json::Value) - rename "file-history-snapshot"
├── QueueOperation(serde_json::Value) - rename "queue-operation"
└── Unknown (serde other)

RawMessage { role: String, content: MessageContent }
MessageContent (untagged enum) { Text(String), Blocks(Vec<ContentBlock>) }
ContentBlock (tagged on "type") { Text { text }, ToolUse { id, name, input }, ToolResult { tool_use_id, content, is_error }, Thinking { thinking }, Unknown }
RawAssistantMessage { model, id, role, content: Vec<ContentBlock>, stop_reason, usage: Option<RawUsage> }
RawUsage { input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens }
RawCompactMetadata { trigger: String, pre_tokens: u64 }
```

**Step 3: Run tests**

Run: `cargo test parser::raw`
Expected: PASS

**Step 4: Commit**

```bash
git add src/parser/
git commit -m "feat: raw JSONL deserialization with serde"
```

---

### Task 4: Raw -> Block Transformation + Session Building

**Files:**
- Create: `src/parser/transform.rs`
- Modify: `src/parser/mod.rs`
- Create: `src/model/session.rs`
- Modify: `src/model/mod.rs`

This task handles the core transformation logic AND session building together, since they're tightly coupled. The transformer reads raw entries and produces blocks + a Session with the DAG.

**Step 1: Write failing tests for transformation**

Tests in `src/parser/transform.rs`:

1. `transform_user_text_message` - raw User with text content -> UserBlock
2. `transform_user_meta_skipped_as_user` - raw User with is_meta=true -> UserBlock with is_meta=true
3. `transform_assistant_text` - raw Assistant with text content -> AssistantBlock with content
4. `transform_assistant_with_thinking` - raw Assistant with thinking block -> AssistantBlock with thinking
5. `transform_assistant_with_tool_calls` - raw Assistant with tool_use blocks -> AssistantBlock with tool_calls vec
6. `transform_tool_result_pairs_with_tool_use` - sequence: assistant(tool_use) then user(tool_result) -> produces ToolBlock with output
7. `transform_assistant_merge_by_request_id` - two assistant entries with same request_id -> single AssistantBlock with merged content and tool_calls
8. `transform_system_turn_duration` - raw System with turn_duration -> SystemBlock
9. `transform_system_compact_boundary` - raw System with compact_boundary -> SystemBlock
10. `transform_skips_progress_and_snapshots` - progress and file-history-snapshot entries produce no blocks
11. `transform_token_merging` - merged assistant entries sum their token counts

Tests in `src/model/session.rs`:

1. `session_builds_dag_from_blocks` - build session, verify parent/children relationships
2. `session_uuid_index` - verify UUID lookup works
3. `session_type_index` - verify blocks_of_type returns correct blocks
4. `session_tool_name_index` - verify tools_by_name works
5. `session_chronological_order` - verify chronological vec is timestamp-sorted
6. `session_roots` - verify root nodes (no parent) are identified
7. `session_children_of` - verify children lookup
8. `session_subtree` - verify DFS subtree traversal

**Step 2: Implement the transformer**

The transformer maintains state as it processes entries in order:
- `pending_tool_calls: HashMap<String, ToolCall>` - tool_use_id -> ToolCall, waiting for tool_result
- `assistant_by_request_id: HashMap<String, usize>` - request_id -> index into result vec, for merging

Top-level function:
```rust
pub fn parse_session(path: &Path) -> anyhow::Result<Session>
```

Steps:
1. Read file line by line, tracking byte offsets for provenance
2. Deserialize each line into RawEntry
3. Transform RawEntry -> Option<Block> (with merging/pairing state)
4. Build Session from resulting blocks

The Session builder (`Session::build(blocks, provenance)`) constructs:
- uuid_index from each block's uuid
- parent/children maps from parent_uuid references
- by_type and by_tool_name secondary indexes
- chronological sorted vec
- roots (blocks with no parent or orphaned parent_uuid)

**Step 3: Implement Session struct**

```rust
pub struct Session {
    pub blocks: Vec<Block>,
    uuid_index: HashMap<String, NodeId>,
    children: HashMap<NodeId, Vec<NodeId>>,
    parent_map: HashMap<NodeId, NodeId>,
    pub roots: Vec<NodeId>,
    by_type: HashMap<BlockType, Vec<NodeId>>,
    by_tool_name: HashMap<String, Vec<NodeId>>,
    by_request_id: HashMap<String, Vec<NodeId>>,
    pub chronological: Vec<NodeId>,
    pub provenance: HashMap<NodeId, Vec<RawLineRef>>,
    pub session_id: Option<String>,
    pub file_path: String,
}
```

Traversal methods:
- `children_of(NodeId) -> &[NodeId]` (empty slice if none)
- `parent_of(NodeId) -> Option<NodeId>`
- `subtree(NodeId) -> Vec<NodeId>` (DFS)
- `ancestors(NodeId) -> Vec<NodeId>` (walk to root)
- `blocks_of_type(BlockType) -> &[NodeId]` (empty slice if none)
- `tools_by_name(&str) -> &[NodeId]`
- `block(&self, id: NodeId) -> &Block` (index into blocks vec)

**Step 4: Run all tests**

Run: `cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add src/parser/ src/model/
git commit -m "feat: JSONL to block transformation and session DAG construction"
```

---

### Task 5: Integration Tests with Real Data

**Files:**
- Create: `tests/real_data.rs`

**Step 1: Write integration tests**

```rust
use clauson::parser::parse_session;
use clauson::model::types::BlockType;
use std::path::Path;

#[test]
fn parse_small_session() {
    // This file has only user + progress entries, no assistant
    let session = parse_session(Path::new("testdata/421d2e3a-f3d7-4c79-9e04-459471305d6f.jsonl")).unwrap();
    assert!(session.blocks.len() > 0, "should have some blocks");
    // All blocks should be User type (no assistant entries in this file)
    for &id in session.blocks_of_type(BlockType::User) {
        assert_eq!(session.block(id).block_type(), BlockType::User);
    }
}

#[test]
fn parse_medium_session() {
    // e998eca1 has: 58 assistant, 37 user, 29 progress, 4 system entries
    let session = parse_session(Path::new("testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl")).unwrap();

    // Should have blocks of multiple types
    assert!(!session.blocks_of_type(BlockType::Assistant).is_empty());
    assert!(!session.blocks_of_type(BlockType::User).is_empty());
    assert!(!session.blocks_of_type(BlockType::Tool).is_empty());
    assert!(!session.blocks_of_type(BlockType::System).is_empty());

    // Every tool block should have a non-empty tool_name
    for &id in session.blocks_of_type(BlockType::Tool) {
        if let clauson::model::block::Block::Tool(t) = session.block(id) {
            assert!(!t.tool_name.is_empty(), "tool block should have a name");
        }
    }
}

#[test]
fn parse_medium_session_2() {
    let session = parse_session(Path::new("testdata/058c4c27-07c1-4f93-86c1-317a4faa9803.jsonl")).unwrap();
    assert!(session.blocks.len() > 0);
}

#[test]
fn parse_medium_session_3() {
    let session = parse_session(Path::new("testdata/6577be84-6784-4198-b13e-25baaaa2e1d2.jsonl")).unwrap();
    assert!(session.blocks.len() > 0);
}

#[test]
fn parse_medium_session_4() {
    let session = parse_session(Path::new("testdata/f4f38a6b-a385-4732-916a-0312b9455d5f.jsonl")).unwrap();
    assert!(session.blocks.len() > 0);
}

#[test]
fn parse_large_session() {
    // f1cf0635 has 1348 lines, 484 assistant, 308 user, 79 system entries
    let session = parse_session(Path::new("testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl")).unwrap();
    assert!(session.blocks.len() > 100, "large session should have many blocks");

    // Should have system blocks including compact_boundary
    assert!(!session.blocks_of_type(BlockType::System).is_empty());
}

#[test]
fn parse_snapshot_only_file() {
    // 15272057 has ONLY file-history-snapshot entries - should produce empty session
    let session = parse_session(Path::new("testdata/15272057-5296-4b76-a119-e4af992a70e0.jsonl")).unwrap();
    assert_eq!(session.blocks.len(), 0, "snapshot-only file should produce no blocks");
}

#[test]
fn provenance_tracks_line_numbers() {
    let session = parse_session(Path::new("testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl")).unwrap();
    // Every block should have provenance
    for id in 0..session.blocks.len() {
        let prov = session.provenance.get(&id);
        assert!(prov.is_some(), "block {id} should have provenance");
        assert!(!prov.unwrap().is_empty(), "block {id} should have at least one line ref");
        // Line numbers should be >= 1
        for line_ref in prov.unwrap() {
            assert!(line_ref.line_number >= 1);
        }
    }
}

#[test]
fn assistant_request_id_merging() {
    // In the medium session, assistant entries with same requestId should be merged
    let session = parse_session(Path::new("testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl")).unwrap();

    // Count assistant blocks - should be fewer than raw assistant entry count (58)
    // because entries with same requestId get merged
    let assistant_count = session.blocks_of_type(BlockType::Assistant).len();
    assert!(assistant_count < 58, "merged assistant count ({assistant_count}) should be less than raw entry count (58)");
    assert!(assistant_count > 0, "should have some assistant blocks");
}
```

**Step 2: Run tests**

Run: `cargo test --test real_data`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: integration tests with real JSONL session data"
```

---

### Task 6: Turn Detection

**Files:**
- Create: `src/model/turn.rs`
- Modify: `src/model/mod.rs`
- Modify: `src/model/session.rs`

**Step 1: Write failing tests**

Tests in `src/model/turn.rs`:

1. `detect_turns_simple` - session with 2 user prompts should produce 2 turns
2. `turn_contains_all_blocks` - each turn should contain its user block + descendant assistant + tool blocks
3. `turn_aggregates_tokens` - turn's total_tokens should sum all assistant block tokens
4. `turn_tool_result_carriers_dont_start_turns` - user entries that only carry tool_result should not start a new turn
5. `meta_user_messages_dont_start_turns` - user entries with is_meta=true should not start new turns

A Turn is detected by walking chronological blocks and starting a new turn at each "real" UserBlock (not is_meta, not a tool_result carrier).

```rust
#[derive(Debug, Serialize)]
pub struct Turn {
    pub index: usize,              // 0-based turn number
    pub user_block: NodeId,
    pub all_blocks: Vec<NodeId>,
    pub tool_blocks: Vec<NodeId>,
    pub assistant_blocks: Vec<NodeId>,
    pub system_blocks: Vec<NodeId>,
    pub total_tokens: TokenUsage,
    pub duration_ms: Option<u64>,  // from turn_duration system entry if present
}
```

Add to Session:
```rust
pub fn turns(&self) -> Vec<Turn>
```

**Step 2: Implement turn detection**

Walk chronological blocks. A new turn starts when we see a UserBlock that:
- Is not is_meta
- Has actual content (not None)

All subsequent blocks until the next such UserBlock belong to the current turn.

**Step 3: Run tests**

Run: `cargo test model::turn`
Expected: PASS

**Step 4: Add real data turn test**

Add to `tests/real_data.rs`:
```rust
#[test]
fn turns_detected_in_medium_session() {
    let session = parse_session(Path::new("testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl")).unwrap();
    let turns = session.turns();
    assert!(turns.len() >= 1, "should detect at least one turn");
    // Each turn should have a user block
    for turn in &turns {
        assert_eq!(session.block(turn.user_block).block_type(), BlockType::User);
    }
}
```

**Step 5: Commit**

```bash
git add src/model/ tests/
git commit -m "feat: turn detection with token aggregation"
```

---

### Task 7: CLI Scaffolding + `blocks list` Command

**Files:**
- Modify: `src/main.rs`
- Create: `src/cli/mod.rs` (replace stub)
- Create: `src/cli/blocks.rs`
- Create: `src/cli/output.rs`
- Modify: `src/lib.rs`

**Step 1: Set up clap CLI structure**

```rust
// src/cli/mod.rs
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod blocks;
pub mod output;

#[derive(Parser)]
#[command(name = "clauson", about = "Claude session JSONL analyzer")]
pub struct Cli {
    /// Path to the JSONL session file
    pub file: PathBuf,

    #[command(subcommand)]
    pub command: Command,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Show raw JSONL lines instead of parsed blocks
    #[arg(long, global = true)]
    pub raw: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Query and filter blocks
    Blocks {
        #[command(subcommand)]
        action: Option<blocks::BlocksAction>,
    },
    // Future: Tools, Turns, Tokens
}
```

```rust
// src/cli/blocks.rs
use clap::Subcommand;

#[derive(Subcommand)]
pub enum BlocksAction {
    /// List blocks (default)
    List {
        /// Filter by block type
        #[arg(long, value_name = "TYPE")]
        r#type: Option<String>,

        /// Filter by turn number (1-indexed)
        #[arg(long)]
        turn: Option<usize>,

        /// Filter by tool name
        #[arg(long, value_name = "NAME")]
        tool_name: Option<String>,
    },
    // Future: Count, Show
}
```

```rust
// src/cli/output.rs - formatting helpers
```

```rust
// src/main.rs
mod cli;

use clap::Parser;
use cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let session = clauson::parser::parse_session(&cli.file)?;

    match &cli.command {
        cli::Command::Blocks { action } => {
            cli::blocks::run(&session, action.as_ref(), cli.json, cli.raw)?;
        }
    }
    Ok(())
}
```

The `blocks::run` function:
- Default (no action or List) prints a table of blocks: index, type, timestamp, uuid (truncated), summary
- With `--type` filter: only show blocks of that type
- With `--json`: output as JSON array
- With `--raw`: for each matching block, read the raw JSONL lines from the file using provenance

**Step 2: Write e2e test**

Create `tests/cli_tests.rs`:
```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn blocks_list_runs() {
    Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "blocks", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("user"))
        .stdout(predicate::str::contains("assistant"));
}

#[test]
fn blocks_list_filter_by_type() {
    Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "blocks", "list", "--type", "tool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tool"));
}

#[test]
fn blocks_list_json_valid() {
    let output = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "blocks", "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn blocks_default_is_list() {
    // `blocks` without subcommand should behave like `blocks list`
    let with_list = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "blocks", "list"])
        .output().unwrap();
    let without_list = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "blocks"])
        .output().unwrap();
    assert_eq!(with_list.stdout, without_list.stdout);
}
```

**Step 3: Implement and verify**

Run: `cargo test --test cli_tests`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ tests/
git commit -m "feat: CLI scaffolding with blocks list command"
```

---

### Task 8: `blocks count` + `blocks show` Commands

**Files:**
- Modify: `src/cli/blocks.rs`

**Step 1: Add subcommands**

Add to `BlocksAction`:
```rust
Count {
    /// Group by: type, tool
    #[arg(long, value_name = "FIELD", default_value = "type")]
    group_by: String,
},
Show {
    /// Block UUID (prefix match supported)
    uuid: String,
},
```

`blocks count --group-by type` outputs a table:
```
Type       Count
user         15
assistant    12
tool         25
system        4
```

`blocks count --group-by tool` outputs:
```
Tool Name   Count
Read           8
Bash           5
Glob           4
...
```

`blocks show <uuid>` finds the block by UUID prefix match and displays its full details. With `--raw`, shows the raw JSONL lines.

**Step 2: Write e2e tests**

```rust
#[test]
fn blocks_count_by_type() {
    Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "blocks", "count"])
        .assert()
        .success()
        .stdout(predicate::str::contains("user"))
        .stdout(predicate::str::contains("assistant"));
}

#[test]
fn blocks_count_by_tool() {
    Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "blocks", "count", "--group-by", "tool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Read"));
}

#[test]
fn blocks_count_json() {
    let output = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "blocks", "count", "--json"])
        .output().unwrap();
    assert!(output.status.success());
    let _: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
}
```

**Step 3: Implement and verify**

Run: `cargo test --test cli_tests`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ tests/
git commit -m "feat: blocks count and blocks show commands"
```

---

### Task 9: `tools list` Command

**Files:**
- Create: `src/cli/tools.rs`
- Modify: `src/cli/mod.rs`

**Step 1: Add tools subcommand**

```rust
// In Command enum:
Tools {
    #[command(subcommand)]
    action: Option<tools::ToolsAction>,
},
```

`tools list` (default) outputs unique tool names with counts, sorted by count descending:
```
Tool Name   Count   Avg Time
Read           12       45ms
Bash            8      230ms
Glob            5       12ms
Write           3       67ms
```

Options: `--sort count|name`

**Step 2: Write e2e tests**

```rust
#[test]
fn tools_list_shows_unique_tools() {
    Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "tools", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Read"));
}

#[test]
fn tools_list_json() {
    let output = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "tools", "list", "--json"])
        .output().unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn tools_default_is_list() {
    let with_list = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "tools", "list"])
        .output().unwrap();
    let without_list = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "tools"])
        .output().unwrap();
    assert_eq!(with_list.stdout, without_list.stdout);
}
```

**Step 3: Implement and verify**

Run: `cargo test --test cli_tests`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ tests/
git commit -m "feat: tools list command with unique tool counts"
```

---

### Task 10: `tokens summary` + `tokens by-turn` Commands

**Files:**
- Create: `src/cli/tokens.rs`
- Modify: `src/cli/mod.rs`

**Step 1: Add tokens subcommand**

```rust
Tokens {
    #[command(subcommand)]
    action: Option<tokens::TokensAction>,
},
```

`tokens summary` (default) shows aggregate token stats:
```
Token Summary
─────────────────────────
Input tokens:              1,234
Cache creation tokens:     5,678
Cache read tokens:        12,345
Output tokens:             2,456
─────────────────────────
Total:                    21,713
```

`tokens by-turn` shows a table:
```
Turn  User Prompt (truncated)     Input    Cache Create  Cache Read  Output   Total
  1   "Hello, help me with..."      234          1,200       5,000     456    6,890
  2   "Now let's implement..."      345          2,300       8,000     678   11,323
```

**Step 2: Write e2e tests**

```rust
#[test]
fn tokens_summary() {
    Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "tokens"])
        .assert()
        .success()
        .stdout(predicate::str::contains("input"));
}

#[test]
fn tokens_by_turn() {
    Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "tokens", "by-turn"])
        .assert()
        .success();
}

#[test]
fn tokens_summary_json() {
    let output = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "tokens", "--json"])
        .output().unwrap();
    assert!(output.status.success());
    let _: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
}
```

**Step 3: Implement and verify**

Run: `cargo test --test cli_tests`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ tests/
git commit -m "feat: tokens summary and tokens by-turn commands"
```

---

### Task 11: `turns list` + `turns show` Commands

**Files:**
- Create: `src/cli/turns.rs`
- Modify: `src/cli/mod.rs`

**Step 1: Add turns subcommand**

`turns list` shows all turns:
```
Turn  Timestamp             Blocks  Tools  Tokens    Duration    User Prompt
  1   2026-02-18 11:30:24       12      5   6,890      31.8s    "Hello, help me with..."
  2   2026-02-18 11:31:02        8      3   4,200      12.5s    "Now let's implement..."
```

`turns show <N>` shows detailed view of a specific turn: all blocks in the turn listed with their details.

**Step 2: Write e2e tests**

```rust
#[test]
fn turns_list() {
    Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "turns", "list"])
        .assert()
        .success();
}

#[test]
fn turns_list_json() {
    let output = Command::cargo_bin("clauson").unwrap()
        .args(["testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl", "turns", "list", "--json"])
        .output().unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}
```

**Step 3: Implement and verify**

Run: `cargo test --test cli_tests`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ tests/
git commit -m "feat: turns list and turns show commands"
```

---

### Task 12: Polish + Final Verification

**Step 1: Run full test suite**

Run: `cargo test`
Expected: ALL PASS

**Step 2: Test with all real data files manually**

```bash
cargo run -- testdata/421d2e3a-f3d7-4c79-9e04-459471305d6f.jsonl blocks
cargo run -- testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl blocks
cargo run -- testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl blocks count
cargo run -- testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl tools
cargo run -- testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl tokens
cargo run -- testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl tokens by-turn
cargo run -- testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl turns
cargo run -- testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl blocks count
cargo run -- testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl tools
cargo run -- testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl tokens
```

Verify each produces reasonable output without errors.

**Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Fix any warnings.

**Step 4: Commit**

```bash
git add -A
git commit -m "chore: polish and clippy fixes"
```
