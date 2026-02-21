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
