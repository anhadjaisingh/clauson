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

impl std::ops::AddAssign for TokenUsage {
    fn add_assign(&mut self, other: Self) {
        self.merge(&other);
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
    pub line_number: usize, // 1-indexed
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
        let mut a = TokenUsage {
            input_tokens: 10,
            output_tokens: 5,
            ..Default::default()
        };
        let b = TokenUsage {
            input_tokens: 20,
            output_tokens: 15,
            ..Default::default()
        };
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
