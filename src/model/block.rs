use chrono::{DateTime, Utc};
use serde::Serialize;

use super::types::{BlockType, EntryMetadata, TokenUsage};

/// A tool invocation extracted from an assistant message's content blocks
#[derive(Debug, Clone, Serialize)]
pub struct ToolCall {
    pub tool_use_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
}

/// System entry subtypes
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SystemSubtype {
    TurnDuration { duration_ms: u64 },
    StopHookSummary,
    CompactBoundary { trigger: String, pre_tokens: u64 },
    Other { subtype: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct UserBlock {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub content: Option<String>,
    pub is_meta: bool,
    pub metadata: EntryMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssistantBlock {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub request_id: String,
    pub model: String,
    pub content: Option<String>,
    pub thinking: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub stop_reason: Option<String>,
    pub tokens: TokenUsage,
    pub metadata: EntryMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolBlock {
    pub tool_use_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub is_error: bool,
    pub assistant_uuid: String,
    pub result_uuid: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: EntryMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemBlock {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub subtype: SystemSubtype,
    pub metadata: EntryMetadata,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "block_type", rename_all = "snake_case")]
pub enum Block {
    User(UserBlock),
    Assistant(AssistantBlock),
    Tool(ToolBlock),
    System(SystemBlock),
}

/// Trait for generic querying across block types
pub trait BlockInfo {
    fn block_type(&self) -> BlockType;
    fn timestamp(&self) -> DateTime<Utc>;
    fn uuid(&self) -> &str;
    fn parent_uuid(&self) -> Option<&str>;
    fn tokens(&self) -> Option<&TokenUsage>;
    fn duration_ms(&self) -> Option<u64>;
}

impl BlockInfo for Block {
    fn block_type(&self) -> BlockType {
        match self {
            Block::User(_) => BlockType::User,
            Block::Assistant(_) => BlockType::Assistant,
            Block::Tool(_) => BlockType::Tool,
            Block::System(_) => BlockType::System,
        }
    }

    fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Block::User(b) => b.timestamp,
            Block::Assistant(b) => b.timestamp,
            Block::Tool(b) => b.timestamp,
            Block::System(b) => b.timestamp,
        }
    }

    fn uuid(&self) -> &str {
        match self {
            Block::User(b) => &b.uuid,
            Block::Assistant(b) => &b.uuid,
            Block::Tool(b) => &b.tool_use_id,
            Block::System(b) => &b.uuid,
        }
    }

    fn parent_uuid(&self) -> Option<&str> {
        match self {
            Block::User(b) => b.parent_uuid.as_deref(),
            Block::Assistant(b) => b.parent_uuid.as_deref(),
            Block::Tool(b) => Some(b.assistant_uuid.as_str()),
            Block::System(b) => b.parent_uuid.as_deref(),
        }
    }

    fn tokens(&self) -> Option<&TokenUsage> {
        match self {
            Block::Assistant(b) => Some(&b.tokens),
            _ => None,
        }
    }

    fn duration_ms(&self) -> Option<u64> {
        match self {
            Block::System(b) => match &b.subtype {
                SystemSubtype::TurnDuration { duration_ms } => Some(*duration_ms),
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::TokenUsage;

    fn make_user_block() -> Block {
        Block::User(UserBlock {
            uuid: "user-1".to_string(),
            parent_uuid: None,
            timestamp: Utc::now(),
            session_id: "sess-1".to_string(),
            content: Some("Hello".to_string()),
            is_meta: false,
            metadata: Default::default(),
        })
    }

    fn make_assistant_block() -> Block {
        Block::Assistant(AssistantBlock {
            uuid: "asst-1".to_string(),
            parent_uuid: Some("user-1".to_string()),
            timestamp: Utc::now(),
            session_id: "sess-1".to_string(),
            request_id: "req-1".to_string(),
            model: "claude-opus-4-6".to_string(),
            content: Some("Hi there".to_string()),
            thinking: None,
            tool_calls: vec![],
            stop_reason: Some("end_turn".to_string()),
            tokens: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_input_tokens: 200,
                cache_read_input_tokens: 300,
            },
            metadata: Default::default(),
        })
    }

    fn make_tool_block() -> Block {
        Block::Tool(ToolBlock {
            tool_use_id: "toolu_01".to_string(),
            tool_name: "Read".to_string(),
            input: serde_json::json!({"file_path": "/tmp/test"}),
            output: Some(serde_json::json!("file contents")),
            is_error: false,
            assistant_uuid: "asst-1".to_string(),
            result_uuid: Some("user-2".to_string()),
            timestamp: Utc::now(),
            metadata: Default::default(),
        })
    }

    fn make_system_block() -> Block {
        Block::System(SystemBlock {
            uuid: "sys-1".to_string(),
            parent_uuid: None,
            timestamp: Utc::now(),
            session_id: "sess-1".to_string(),
            subtype: SystemSubtype::TurnDuration {
                duration_ms: 12500,
            },
            metadata: Default::default(),
        })
    }

    #[test]
    fn block_info_user() {
        let block = make_user_block();
        assert_eq!(block.block_type(), BlockType::User);
        assert!(block.tokens().is_none());
        assert_eq!(block.uuid(), "user-1");
        assert!(block.parent_uuid().is_none());
        assert!(block.duration_ms().is_none());
    }

    #[test]
    fn block_info_assistant() {
        let block = make_assistant_block();
        assert_eq!(block.block_type(), BlockType::Assistant);
        let tokens = block.tokens().unwrap();
        assert_eq!(tokens.input_tokens, 100);
        assert_eq!(tokens.output_tokens, 50);
        assert_eq!(block.uuid(), "asst-1");
        assert_eq!(block.parent_uuid(), Some("user-1"));
    }

    #[test]
    fn block_info_tool() {
        let block = make_tool_block();
        assert_eq!(block.block_type(), BlockType::Tool);
        assert!(block.tokens().is_none());
        assert_eq!(block.uuid(), "toolu_01");
        assert_eq!(block.parent_uuid(), Some("asst-1"));
    }

    #[test]
    fn block_info_system_turn_duration() {
        let block = make_system_block();
        assert_eq!(block.block_type(), BlockType::System);
        assert_eq!(block.duration_ms(), Some(12500));
        assert_eq!(block.uuid(), "sys-1");
    }
}
