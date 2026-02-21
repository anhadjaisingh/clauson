# Clauson Design

A Rust CLI for searching, querying, and analyzing Claude Code session JSONL files.

Reference: [claude-tracer#22](https://github.com/anhadjaisingh/claude-tracer/issues/22)

## Goals

- Parse Claude Code session JSONL into a typed DAG
- Provide subcommand-based CLI for filtering, querying, and analyzing sessions
- Be fast, correct, and extensible to new block types and queries
- Human-readable output by default, JSON with `--json`

## Non-goals (v1)

- Sub-agent / sidechain JSONL support (designed for, not implemented)
- Query language / DSL
- Interactive REPL

---

## Data Model

### Two-layer architecture

**Raw layer:** Each JSONL line deserializes into a `RawEntry` enum that closely mirrors the file format. Minimal transformation — just typed serde.

```rust
enum RawEntry {
    User { uuid, parent_uuid, timestamp, session_id, message, metadata, ... },
    Assistant { uuid, parent_uuid, timestamp, session_id, request_id, message, metadata, ... },
    Progress { uuid, parent_uuid, timestamp, data, ... },
    System { uuid, parent_uuid, timestamp, subtype, ... },
    FileHistorySnapshot { message_id, snapshot, ... },
    QueueOperation { ... },
}
```

**Semantic layer:** A second pass transforms `RawEntry` values into `Block` — the query-facing model. This is where merging (assistant entries with same `request_id`), pairing (tool_use with tool_result), and synthesis (ToolBlocks derived from two raw entries) happen.

```rust
enum Block {
    User(UserBlock),
    Assistant(AssistantBlock),
    Tool(ToolBlock),
    System(SystemBlock),
    // Future: Mcp(McpBlock), TeamMessage(TeamMessageBlock), Compaction(CompactionBlock), ...
}
```

### Block variants

```rust
struct UserBlock {
    uuid: String,
    parent_uuid: Option<String>,
    timestamp: DateTime<Utc>,
    session_id: String,
    content: Option<String>,     // actual user message text
    metadata: EntryMetadata,     // version, cwd, git_branch
}

struct AssistantBlock {
    uuid: String,
    parent_uuid: Option<String>,
    timestamp: DateTime<Utc>,
    session_id: String,
    request_id: String,
    model: String,               // "claude-opus-4-6", etc.
    content: Option<String>,     // merged text from all text blocks
    thinking: Option<String>,    // from thinking blocks
    tool_calls: Vec<ToolCall>,   // name + id + input
    stop_reason: Option<String>,
    tokens: TokenUsage,
    metadata: EntryMetadata,
}

struct ToolBlock {
    tool_use_id: String,         // "toolu_0176XTM5..."
    tool_name: String,           // "Read", "Bash", "Glob", etc.
    input: serde_json::Value,
    output: Option<serde_json::Value>,
    is_error: bool,
    assistant_uuid: String,      // which assistant block called this
    result_uuid: Option<String>, // the user entry that carried the result
    timestamp: DateTime<Utc>,
}

struct SystemBlock {
    uuid: String,
    parent_uuid: Option<String>,
    timestamp: DateTime<Utc>,
    subtype: SystemSubtype,      // TurnDuration { duration_ms } | StopHookSummary | ...
}

struct TokenUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
}
```

### BlockInfo trait

All variants implement `BlockInfo` for generic querying:

```rust
trait BlockInfo {
    fn block_type(&self) -> BlockType;
    fn timestamp(&self) -> DateTime<Utc>;
    fn uuid(&self) -> &str;
    fn parent_uuid(&self) -> Option<&str>;
    fn tokens(&self) -> Option<&TokenUsage>;
    fn duration_ms(&self) -> Option<u64>;
}
```

New block types implement this trait and existing queries work automatically.

### Key transformation rules

- **Assistant entries with the same `request_id`** merge into one `AssistantBlock` — text, thinking, tool_calls, and token counts are combined.
- **ToolBlocks are synthesized** by pairing `tool_use` content (in an assistant entry) with the corresponding `tool_result` content (in a subsequent user entry), matched on `tool_use_id`.
- **User entries that are just tool_result carriers** do not become `UserBlock`s — they are absorbed into the `ToolBlock`.
- **Progress, FileHistorySnapshot, QueueOperation** entries are skipped (not converted to blocks). They exist in the JSONL for audit/replay but are not needed for analysis.

---

## Session & DAG

```rust
type NodeId = usize;

struct Session {
    // Arena storage
    blocks: Vec<Block>,

    // UUID -> NodeId
    uuid_index: HashMap<String, NodeId>,

    // DAG edges
    children: HashMap<NodeId, Vec<NodeId>>,
    parent: HashMap<NodeId, NodeId>,
    roots: Vec<NodeId>,

    // Secondary indexes
    by_type: HashMap<BlockType, Vec<NodeId>>,
    by_tool_name: HashMap<String, Vec<NodeId>>,
    by_request_id: HashMap<String, Vec<NodeId>>,
    chronological: Vec<NodeId>,  // sorted by timestamp

    // Raw provenance: block -> JSONL lines that produced it
    provenance: HashMap<NodeId, Vec<RawLineRef>>,

    // Session metadata
    session_id: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

struct RawLineRef {
    line_number: usize,   // 1-indexed
    byte_offset: usize,
    byte_length: usize,
}
```

Provenance is a `Vec<RawLineRef>` because a single block can be produced from multiple raw lines (merged assistant entries, synthesized tool blocks). Enables `--raw` flag to dump original JSONL lines for any block.

### Turns

A **turn** starts at a `UserBlock` whose content is an actual user message (not a tool_result carrier) and extends through all descendants until the next such `UserBlock`.

```rust
struct Turn {
    user_block: NodeId,
    all_blocks: Vec<NodeId>,
    tool_calls: Vec<NodeId>,
    assistant_blocks: Vec<NodeId>,
    total_tokens: TokenUsage,
    total_duration_ms: u64,
}
```

### Traversal API

```rust
impl Session {
    fn children_of(&self, node: NodeId) -> &[NodeId];
    fn parent_of(&self, node: NodeId) -> Option<NodeId>;
    fn subtree(&self, node: NodeId) -> Vec<NodeId>;
    fn ancestors(&self, node: NodeId) -> Vec<NodeId>;

    fn blocks_of_type(&self, t: BlockType) -> &[NodeId];
    fn tools_by_name(&self, name: &str) -> &[NodeId];

    fn turns(&self) -> Vec<Turn>;
    fn turn_for_block(&self, node: NodeId) -> Option<&Turn>;
}
```

### Sub-agent extension (future)

When we add sub-agent support, the model extends naturally:

```rust
struct Session {
    // ... existing fields ...
    subagents: HashMap<String, Session>,           // agent_id -> its own DAG
    agent_dispatches: HashMap<NodeId, String>,     // Task ToolBlock -> agent_id
}
```

Each sub-agent JSONL becomes its own `Session`. The main session's `Task` tool calls link to sub-agent sessions via `agent_id`. Queries across sub-agents iterate `session.subagents` and aggregate.

Team-mate agents add a `TeamMessage(TeamMessageBlock)` variant to the `Block` enum with sender, recipient, content, and message_type fields.

---

## CLI Design

Noun-verb subcommand structure (like `gh`, `kubectl`).

```
clauson <FILE> <NOUN> <VERB> [OPTIONS]

Global flags:
  --json    Output as JSON
  --raw     Show raw JSONL lines (where applicable)
```

### `blocks`

```
clauson <file> blocks list [--type user|assistant|tool|system] [--turn N] [--tool-name NAME]
clauson <file> blocks count [--group-by type|tool]
clauson <file> blocks show <UUID>
```

### `tools`

```
clauson <file> tools list [--sort count|name]
clauson <file> tools freq [--bucket 60]
```

### `turns`

```
clauson <file> turns list
clauson <file> turns show <N>
clauson <file> turns stats [--turn N]
```

### `tokens`

```
clauson <file> tokens summary
clauson <file> tokens by-turn
clauson <file> tokens by-block [--type assistant]
```

Each noun's default verb is `list` (or `summary` for tokens), so `clauson <file> blocks` is shorthand for `clauson <file> blocks list`.

---

## Module Layout

```
src/
├── main.rs              # CLI entry, clap arg parsing, dispatch
├── cli/
│   ├── mod.rs           # Clap structs (App, subcommands, flags)
│   ├── blocks.rs        # blocks list/count/show handlers
│   ├── tools.rs         # tools list/freq handlers
│   ├── turns.rs         # turns list/show/stats handlers
│   ├── tokens.rs        # tokens summary/by-turn/by-block handlers
│   └── output.rs        # Human-readable vs JSON formatting
├── parser/
│   ├── mod.rs           # parse_session(path) -> Session
│   ├── raw.rs           # RawEntry enum, serde deserialization
│   ├── transform.rs     # RawEntry -> Block (merging, pairing, synthesis)
│   └── provenance.rs    # RawLineRef tracking during parse
├── model/
│   ├── mod.rs
│   ├── block.rs         # Block enum, variant structs, BlockInfo trait
│   ├── session.rs       # Session struct, DAG construction, indexes
│   ├── turn.rs          # Turn detection and Turn struct
│   └── types.rs         # BlockType, TokenUsage, EntryMetadata, shared types
└── lib.rs               # Re-exports parser + model for library use
```

- `parser/` handles I/O and transformation; `model/` is pure data and traversal.
- `cli/` handlers are thin — call Session methods, format output.
- `lib.rs` re-exports so clauson can be used as a library crate.

---

## Testing Strategy

### Tier 1: Unit tests with synthetic JSONL

Inline handwritten JSON strings testing one thing at a time: parsing each entry type, request_id merging, tool_use/tool_result pairing, DAG construction, index correctness, edge cases (orphaned parentUuid, missing fields, unknown entry types).

### Tier 2: Integration tests with real test data

Load actual JSONL files from `testdata/` and assert structural invariants: all non-root blocks have parents, tool blocks have names, token counts are non-negative, sessions parse without panic.

Test data files (copied from claude-tracer):
- Small: `421d2e3a-f3d7-4c79-9e04-459471305d6f.jsonl` (6KB)
- Medium: `e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl` (189KB)
- Large: `f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl` (4.6MB, with subagents)

### Tier 3: CLI end-to-end tests

Run the binary with `assert_cmd`, check stdout for expected content and valid JSON when `--json` is used.

### Test helpers

Builder functions for constructing synthetic sessions without writing raw JSON: `make_user_block()`, `make_assistant_block()`, `make_tool_result()`, `make_session()`.

### TDD flow

For each feature: write a failing Tier 1 test, implement, then add Tier 2 test against real data. Tier 3 tests added as CLI commands are wired up.

---

## Dependencies (expected)

- `clap` — CLI argument parsing
- `serde`, `serde_json` — JSONL deserialization
- `chrono` — timestamp handling
- `comfy-table` or `tabled` — human-readable table output
- `assert_cmd` — CLI integration tests
