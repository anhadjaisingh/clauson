use serde::Serialize;

use super::types::{NodeId, TokenUsage};

#[derive(Debug, Serialize)]
pub struct Turn {
    pub index: usize,
    pub user_block: NodeId,
    pub all_blocks: Vec<NodeId>,
    pub tool_blocks: Vec<NodeId>,
    pub assistant_blocks: Vec<NodeId>,
    pub system_blocks: Vec<NodeId>,
    pub total_tokens: TokenUsage,
    pub duration_ms: Option<u64>,
}
