use std::collections::HashMap;

use super::block::{AssistantBlock, Block, BlockInfo, ToolBlock};
use super::turn::Turn;
use super::types::{BlockType, NodeId, RawLineRef, TokenUsage};

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

impl Session {
    pub fn build(
        blocks: Vec<Block>,
        provenance: HashMap<NodeId, Vec<RawLineRef>>,
        session_id: Option<String>,
        file_path: String,
    ) -> Self {
        let mut uuid_index = HashMap::new();
        let mut children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut parent_map = HashMap::new();
        let mut roots = Vec::new();
        let mut by_type: HashMap<BlockType, Vec<NodeId>> = HashMap::new();
        let mut by_tool_name: HashMap<String, Vec<NodeId>> = HashMap::new();
        let mut by_request_id: HashMap<String, Vec<NodeId>> = HashMap::new();

        for (id, block) in blocks.iter().enumerate() {
            uuid_index.insert(block.uuid().to_string(), id);
        }

        for (id, block) in blocks.iter().enumerate() {
            if let Some(parent_uuid) = block.parent_uuid() {
                if let Some(&parent_id) = uuid_index.get(parent_uuid) {
                    parent_map.insert(id, parent_id);
                    children.entry(parent_id).or_default().push(id);
                } else {
                    roots.push(id);
                }
            } else {
                roots.push(id);
            }

            by_type.entry(block.block_type()).or_default().push(id);

            if let Block::Tool(ToolBlock { tool_name, .. }) = block {
                by_tool_name.entry(tool_name.clone()).or_default().push(id);
            }

            if let Block::Assistant(AssistantBlock { request_id, .. }) = block {
                by_request_id
                    .entry(request_id.clone())
                    .or_default()
                    .push(id);
            }
        }

        let mut chronological: Vec<NodeId> = (0..blocks.len()).collect();
        chronological.sort_by_key(|&id| blocks[id].timestamp());

        Session {
            blocks,
            uuid_index,
            children,
            parent_map,
            roots,
            by_type,
            by_tool_name,
            by_request_id,
            chronological,
            provenance,
            session_id,
            file_path,
        }
    }

    pub fn block(&self, id: NodeId) -> &Block {
        &self.blocks[id]
    }

    pub fn children_of(&self, node: NodeId) -> &[NodeId] {
        self.children
            .get(&node)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn parent_of(&self, node: NodeId) -> Option<NodeId> {
        self.parent_map.get(&node).copied()
    }

    pub fn subtree(&self, node: NodeId) -> Vec<NodeId> {
        let mut result = vec![node];
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            for &child in self.children_of(current) {
                result.push(child);
                stack.push(child);
            }
        }
        result
    }

    pub fn ancestors(&self, node: NodeId) -> Vec<NodeId> {
        let mut result = vec![];
        let mut current = node;
        while let Some(parent) = self.parent_of(current) {
            result.push(parent);
            current = parent;
        }
        result
    }

    pub fn blocks_of_type(&self, t: BlockType) -> &[NodeId] {
        self.by_type
            .get(&t)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn tools_by_name(&self, name: &str) -> &[NodeId] {
        self.by_tool_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn node_for_uuid(&self, uuid: &str) -> Option<NodeId> {
        self.uuid_index.get(uuid).copied()
    }

    pub fn turns(&self) -> Vec<Turn> {
        let mut turns = Vec::new();
        let mut current_turn: Option<TurnBuilder> = None;

        for &id in &self.chronological {
            let block = self.block(id);

            if let Block::User(user) = block {
                if !user.is_meta && user.content.is_some() {
                    if let Some(builder) = current_turn.take() {
                        turns.push(builder.finish(turns.len()));
                    }
                    current_turn = Some(TurnBuilder::new(id));
                    continue;
                }
            }

            if let Some(ref mut builder) = current_turn {
                builder.add_block(id, block);
            }
        }

        if let Some(builder) = current_turn {
            turns.push(builder.finish(turns.len()));
        }

        turns
    }
}

struct TurnBuilder {
    user_block: NodeId,
    all_blocks: Vec<NodeId>,
    tool_blocks: Vec<NodeId>,
    assistant_blocks: Vec<NodeId>,
    system_blocks: Vec<NodeId>,
    total_tokens: TokenUsage,
    duration_ms: Option<u64>,
}

impl TurnBuilder {
    fn new(user_block: NodeId) -> Self {
        TurnBuilder {
            user_block,
            all_blocks: vec![user_block],
            tool_blocks: vec![],
            assistant_blocks: vec![],
            system_blocks: vec![],
            total_tokens: TokenUsage::default(),
            duration_ms: None,
        }
    }

    fn add_block(&mut self, id: NodeId, block: &Block) {
        self.all_blocks.push(id);
        match block {
            Block::Assistant(a) => {
                self.assistant_blocks.push(id);
                self.total_tokens.merge(&a.tokens);
            }
            Block::Tool(_) => {
                self.tool_blocks.push(id);
            }
            Block::System(s) => {
                self.system_blocks.push(id);
                if let super::block::SystemSubtype::TurnDuration { duration_ms } = &s.subtype {
                    self.duration_ms = Some(*duration_ms);
                }
            }
            Block::User(_) => {}
        }
    }

    fn finish(self, index: usize) -> Turn {
        Turn {
            index,
            user_block: self.user_block,
            all_blocks: self.all_blocks,
            tool_blocks: self.tool_blocks,
            assistant_blocks: self.assistant_blocks,
            system_blocks: self.system_blocks,
            total_tokens: self.total_tokens,
            duration_ms: self.duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::block::*;
    use crate::model::types::*;
    use chrono::{TimeZone, Utc};

    fn ts(secs: i64) -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(1700000000 + secs, 0).unwrap()
    }

    fn make_user(uuid: &str, parent: Option<&str>, content: &str, t: chrono::DateTime<Utc>) -> Block {
        Block::User(UserBlock {
            uuid: uuid.to_string(),
            parent_uuid: parent.map(|s| s.to_string()),
            timestamp: t,
            session_id: "test-session".to_string(),
            content: Some(content.to_string()),
            is_meta: false,
            metadata: Default::default(),
        })
    }

    fn make_user_meta(uuid: &str, parent: Option<&str>, t: chrono::DateTime<Utc>) -> Block {
        Block::User(UserBlock {
            uuid: uuid.to_string(),
            parent_uuid: parent.map(|s| s.to_string()),
            timestamp: t,
            session_id: "test-session".to_string(),
            content: Some("meta content".to_string()),
            is_meta: true,
            metadata: Default::default(),
        })
    }

    fn make_user_no_content(uuid: &str, parent: Option<&str>, t: chrono::DateTime<Utc>) -> Block {
        Block::User(UserBlock {
            uuid: uuid.to_string(),
            parent_uuid: parent.map(|s| s.to_string()),
            timestamp: t,
            session_id: "test-session".to_string(),
            content: None,
            is_meta: false,
            metadata: Default::default(),
        })
    }

    fn make_assistant(
        uuid: &str,
        parent: Option<&str>,
        request_id: &str,
        t: chrono::DateTime<Utc>,
        tokens: TokenUsage,
    ) -> Block {
        Block::Assistant(AssistantBlock {
            uuid: uuid.to_string(),
            parent_uuid: parent.map(|s| s.to_string()),
            timestamp: t,
            session_id: "test-session".to_string(),
            request_id: request_id.to_string(),
            model: "test-model".to_string(),
            content: Some("response".to_string()),
            thinking: None,
            tool_calls: vec![],
            stop_reason: Some("end_turn".to_string()),
            tokens,
            metadata: Default::default(),
        })
    }

    fn make_tool(id: &str, name: &str, assistant_uuid: &str, t: chrono::DateTime<Utc>) -> Block {
        Block::Tool(ToolBlock {
            tool_use_id: id.to_string(),
            tool_name: name.to_string(),
            input: serde_json::json!({}),
            output: Some(serde_json::json!("result")),
            is_error: false,
            assistant_uuid: assistant_uuid.to_string(),
            result_uuid: None,
            timestamp: t,
            metadata: Default::default(),
        })
    }

    fn make_system_turn_duration(
        uuid: &str,
        parent: Option<&str>,
        duration_ms: u64,
        t: chrono::DateTime<Utc>,
    ) -> Block {
        Block::System(SystemBlock {
            uuid: uuid.to_string(),
            parent_uuid: parent.map(|s| s.to_string()),
            timestamp: t,
            session_id: "test-session".to_string(),
            subtype: SystemSubtype::TurnDuration { duration_ms },
            metadata: Default::default(),
        })
    }

    fn build_session(blocks: Vec<Block>) -> Session {
        Session::build(blocks, HashMap::new(), Some("test".to_string()), "test.jsonl".to_string())
    }

    #[test]
    fn session_builds_dag_from_blocks() {
        let blocks = vec![
            make_user("u1", None, "hello", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
            make_tool("t1", "Read", "a1", ts(2)),
        ];
        let session = build_session(blocks);
        assert_eq!(session.children_of(0), &[1]);
        assert_eq!(session.parent_of(1), Some(0));
        assert_eq!(session.parent_of(2), Some(1));
    }

    #[test]
    fn session_uuid_index() {
        let blocks = vec![
            make_user("u1", None, "hello", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
        ];
        let session = build_session(blocks);
        assert_eq!(session.node_for_uuid("u1"), Some(0));
        assert_eq!(session.node_for_uuid("a1"), Some(1));
        assert_eq!(session.node_for_uuid("nonexistent"), None);
    }

    #[test]
    fn session_type_index() {
        let blocks = vec![
            make_user("u1", None, "hello", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
            make_tool("t1", "Read", "a1", ts(2)),
        ];
        let session = build_session(blocks);
        assert_eq!(session.blocks_of_type(BlockType::User), &[0]);
        assert_eq!(session.blocks_of_type(BlockType::Assistant), &[1]);
        assert_eq!(session.blocks_of_type(BlockType::Tool), &[2]);
        assert!(session.blocks_of_type(BlockType::System).is_empty());
    }

    #[test]
    fn session_tool_name_index() {
        let blocks = vec![
            make_tool("t1", "Read", "a1", ts(0)),
            make_tool("t2", "Bash", "a1", ts(1)),
            make_tool("t3", "Read", "a1", ts(2)),
        ];
        let session = build_session(blocks);
        assert_eq!(session.tools_by_name("Read"), &[0, 2]);
        assert_eq!(session.tools_by_name("Bash"), &[1]);
        assert!(session.tools_by_name("Glob").is_empty());
    }

    #[test]
    fn session_chronological_order() {
        let blocks = vec![
            make_user("u1", None, "third", ts(30)),
            make_user("u2", None, "first", ts(10)),
            make_user("u3", None, "second", ts(20)),
        ];
        let session = build_session(blocks);
        assert_eq!(session.chronological, vec![1, 2, 0]);
    }

    #[test]
    fn session_roots() {
        let blocks = vec![
            make_user("u1", None, "root1", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
            make_user("u2", None, "root2", ts(2)),
        ];
        let session = build_session(blocks);
        assert_eq!(session.roots, vec![0, 2]);
    }

    #[test]
    fn session_subtree() {
        let blocks = vec![
            make_user("u1", None, "root", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
            make_tool("t1", "Read", "a1", ts(2)),
        ];
        let session = build_session(blocks);
        let subtree = session.subtree(0);
        assert!(subtree.contains(&0));
        assert!(subtree.contains(&1));
        assert!(subtree.contains(&2));
        assert_eq!(subtree.len(), 3);
    }

    #[test]
    fn session_ancestors() {
        let blocks = vec![
            make_user("u1", None, "root", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
            make_tool("t1", "Read", "a1", ts(2)),
        ];
        let session = build_session(blocks);
        let ancestors = session.ancestors(2);
        assert_eq!(ancestors, vec![1, 0]);
    }

    #[test]
    fn session_orphaned_parent_becomes_root() {
        let blocks = vec![
            make_user("u1", None, "root", ts(0)),
            make_assistant("a1", Some("nonexistent"), "req1", ts(1), TokenUsage::default()),
        ];
        let session = build_session(blocks);
        assert!(session.roots.contains(&0));
        assert!(session.roots.contains(&1));
    }

    #[test]
    fn detect_turns_simple() {
        let blocks = vec![
            make_user("u1", None, "first prompt", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
            make_user("u2", None, "second prompt", ts(10)),
            make_assistant("a2", Some("u2"), "req2", ts(11), TokenUsage::default()),
        ];
        let session = build_session(blocks);
        let turns = session.turns();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].index, 0);
        assert_eq!(turns[1].index, 1);
    }

    #[test]
    fn turn_contains_all_blocks() {
        let tokens = TokenUsage { input_tokens: 100, output_tokens: 50, ..Default::default() };
        let blocks = vec![
            make_user("u1", None, "hello", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), tokens),
            make_tool("t1", "Read", "a1", ts(2)),
            make_system_turn_duration("s1", Some("u1"), 5000, ts(3)),
        ];
        let session = build_session(blocks);
        let turns = session.turns();
        assert_eq!(turns.len(), 1);
        let turn = &turns[0];
        assert_eq!(turn.user_block, 0);
        assert_eq!(turn.assistant_blocks, vec![1]);
        assert_eq!(turn.tool_blocks, vec![2]);
        assert_eq!(turn.system_blocks, vec![3]);
        assert_eq!(turn.all_blocks.len(), 4);
    }

    #[test]
    fn turn_aggregates_tokens() {
        let t1 = TokenUsage { input_tokens: 100, output_tokens: 50, cache_creation_input_tokens: 200, cache_read_input_tokens: 300 };
        let t2 = TokenUsage { input_tokens: 10, output_tokens: 5, cache_creation_input_tokens: 20, cache_read_input_tokens: 30 };
        let blocks = vec![
            make_user("u1", None, "hello", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), t1),
            make_assistant("a2", Some("a1"), "req2", ts(2), t2),
        ];
        let session = build_session(blocks);
        let turns = session.turns();
        assert_eq!(turns[0].total_tokens.input_tokens, 110);
        assert_eq!(turns[0].total_tokens.output_tokens, 55);
    }

    #[test]
    fn turn_tool_result_carriers_dont_start_turns() {
        let blocks = vec![
            make_user("u1", None, "hello", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
            make_user_no_content("u2", Some("a1"), ts(2)),
            make_user("u3", None, "second prompt", ts(10)),
        ];
        let session = build_session(blocks);
        let turns = session.turns();
        assert_eq!(turns.len(), 2);
        assert!(turns[0].all_blocks.contains(&2));
    }

    #[test]
    fn meta_user_messages_dont_start_turns() {
        let blocks = vec![
            make_user("u1", None, "hello", ts(0)),
            make_user_meta("u2", Some("u1"), ts(1)),
            make_assistant("a1", Some("u2"), "req1", ts(2), TokenUsage::default()),
        ];
        let session = build_session(blocks);
        let turns = session.turns();
        assert_eq!(turns.len(), 1);
        assert!(turns[0].all_blocks.contains(&1));
    }

    #[test]
    fn turn_captures_duration() {
        let blocks = vec![
            make_user("u1", None, "hello", ts(0)),
            make_assistant("a1", Some("u1"), "req1", ts(1), TokenUsage::default()),
            make_system_turn_duration("s1", Some("u1"), 12500, ts(2)),
        ];
        let session = build_session(blocks);
        let turns = session.turns();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].duration_ms, Some(12500));
    }
}
