use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Top-level entry dispatched by the `type` field.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum RawEntry {
    #[serde(rename = "user")]
    User(RawUserEntry),
    #[serde(rename = "assistant")]
    Assistant(RawAssistantEntry),
    #[serde(rename = "system")]
    System(RawSystemEntry),
    #[serde(rename = "progress")]
    Progress,
    #[serde(rename = "file-history-snapshot")]
    FileHistorySnapshot,
    #[serde(rename = "queue-operation")]
    QueueOperation,
}

/// Fields shared by user, assistant, and system entries.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CommonFields {
    pub uuid: Option<String>,
    pub parent_uuid: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub session_id: Option<String>,
    pub is_sidechain: bool,
    pub version: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub slug: Option<String>,
    pub is_meta: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawUserEntry {
    #[serde(flatten)]
    pub common: CommonFields,
    pub message: RawUserMessage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawUserMessage {
    #[serde(default)]
    pub role: Option<String>,
    pub content: UserContent,
}

/// User message content: either plain text or an array of content blocks.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum UserContent {
    Text(String),
    Blocks(Vec<UserContentBlock>),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum UserContentBlock {
    #[serde(rename = "tool_result")]
    ToolResult(RawToolResult),
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        text: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RawToolResult {
    pub tool_use_id: String,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAssistantEntry {
    #[serde(flatten)]
    pub common: CommonFields,
    #[serde(default)]
    pub request_id: Option<String>,
    pub message: RawAssistantMessage,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RawAssistantMessage {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Vec<AssistantContentBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<RawUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantContentBlock {
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        text: Option<String>,
    },
    #[serde(rename = "thinking")]
    Thinking {
        #[serde(default)]
        thinking: Option<String>,
    },
    #[serde(rename = "tool_use")]
    ToolUse(RawToolUse),
}

#[derive(Debug, Deserialize)]
pub struct RawToolUse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RawUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSystemEntry {
    #[serde(flatten)]
    pub common: CommonFields,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub compact_metadata: Option<RawCompactMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCompactMetadata {
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub pre_tokens: Option<u64>,
}

/// Attempt to deserialize a single JSONL line into a RawEntry.
pub fn parse_line(line: &str) -> Option<RawEntry> {
    serde_json::from_str(line).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_user_text_entry() {
        let json = r#"{"type":"user","uuid":"u1","parentUuid":null,"timestamp":"2026-02-18T14:46:16.829Z","sessionId":"sess1","isSidechain":false,"message":{"role":"user","content":"Hello world"}}"#;
        let entry = parse_line(json).unwrap();
        match entry {
            RawEntry::User(u) => {
                assert_eq!(u.common.uuid.as_deref(), Some("u1"));
                assert!(u.common.parent_uuid.is_none());
                match &u.message.content {
                    UserContent::Text(t) => assert_eq!(t, "Hello world"),
                    _ => panic!("expected text content"),
                }
            }
            _ => panic!("expected user entry"),
        }
    }

    #[test]
    fn parse_user_tool_result_entry() {
        let json = r#"{"type":"user","uuid":"u2","parentUuid":"u1","timestamp":"2026-02-18T14:46:20.000Z","sessionId":"sess1","isSidechain":false,"message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_01","content":"file contents","is_error":false}]}}"#;
        let entry = parse_line(json).unwrap();
        match entry {
            RawEntry::User(u) => match &u.message.content {
                UserContent::Blocks(blocks) => {
                    assert_eq!(blocks.len(), 1);
                    match &blocks[0] {
                        UserContentBlock::ToolResult(tr) => {
                            assert_eq!(tr.tool_use_id, "toolu_01");
                            assert!(!tr.is_error);
                        }
                        _ => panic!("expected tool_result block"),
                    }
                }
                _ => panic!("expected blocks content"),
            },
            _ => panic!("expected user entry"),
        }
    }

    #[test]
    fn parse_assistant_with_text_and_tool_use() {
        let json = r#"{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","id":"msg_01","type":"message","role":"assistant","content":[{"type":"text","text":"Let me read that file."},{"type":"tool_use","id":"toolu_01","name":"Read","input":{"file_path":"/tmp/test"}}],"stop_reason":"tool_use","usage":{"input_tokens":10,"output_tokens":20,"cache_creation_input_tokens":100,"cache_read_input_tokens":200}}}"#;
        let entry = parse_line(json).unwrap();
        match entry {
            RawEntry::Assistant(a) => {
                assert_eq!(a.request_id.as_deref(), Some("req_01"));
                assert_eq!(a.message.content.len(), 2);
                assert_eq!(a.message.stop_reason.as_deref(), Some("tool_use"));
                let usage = a.message.usage.as_ref().unwrap();
                assert_eq!(usage.input_tokens, 10);
                assert_eq!(usage.cache_creation_input_tokens, 100);
            }
            _ => panic!("expected assistant entry"),
        }
    }

    #[test]
    fn parse_assistant_with_thinking() {
        let json = r#"{"type":"assistant","uuid":"a2","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_02","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"thinking","thinking":"Let me think...","signature":"sig"},{"type":"text","text":"Answer."}],"stop_reason":"end_turn"}}"#;
        let entry = parse_line(json).unwrap();
        match entry {
            RawEntry::Assistant(a) => {
                assert_eq!(a.message.content.len(), 2);
                match &a.message.content[0] {
                    AssistantContentBlock::Thinking { thinking } => {
                        assert_eq!(thinking.as_deref(), Some("Let me think..."));
                    }
                    _ => panic!("expected thinking block"),
                }
            }
            _ => panic!("expected assistant entry"),
        }
    }

    #[test]
    fn parse_system_turn_duration() {
        let json = r#"{"type":"system","subtype":"turn_duration","durationMs":12345,"timestamp":"2026-02-18T14:48:50.406Z","uuid":"s1","sessionId":"sess1","isSidechain":false,"isMeta":false}"#;
        let entry = parse_line(json).unwrap();
        match entry {
            RawEntry::System(s) => {
                assert_eq!(s.subtype.as_deref(), Some("turn_duration"));
                assert_eq!(s.duration_ms, Some(12345));
            }
            _ => panic!("expected system entry"),
        }
    }

    #[test]
    fn parse_system_compact_boundary() {
        let json = r#"{"type":"system","subtype":"compact_boundary","timestamp":"2026-02-18T14:18:01.734Z","uuid":"s2","sessionId":"sess1","isSidechain":false,"compactMetadata":{"trigger":"auto","preTokens":169054}}"#;
        let entry = parse_line(json).unwrap();
        match entry {
            RawEntry::System(s) => {
                let cm = s.compact_metadata.as_ref().unwrap();
                assert_eq!(cm.trigger.as_deref(), Some("auto"));
                assert_eq!(cm.pre_tokens, Some(169054));
            }
            _ => panic!("expected system entry"),
        }
    }

    #[test]
    fn parse_progress_entry() {
        let json = r#"{"type":"progress","data":{"type":"hook_progress"},"uuid":"p1","timestamp":"2026-02-18T14:34:31.267Z"}"#;
        let entry = parse_line(json).unwrap();
        assert!(matches!(entry, RawEntry::Progress));
    }

    #[test]
    fn parse_file_history_snapshot() {
        let json = r#"{"type":"file-history-snapshot","messageId":"m1","snapshot":{"messageId":"m1","trackedFileBackups":{},"timestamp":"2026-02-18T14:35:39.242Z"},"isSnapshotUpdate":false}"#;
        let entry = parse_line(json).unwrap();
        assert!(matches!(entry, RawEntry::FileHistorySnapshot));
    }

    #[test]
    fn parse_queue_operation() {
        let json = r#"{"type":"queue-operation","operation":"enqueue","timestamp":"2026-02-18T13:18:55.685Z","content":"some text"}"#;
        let entry = parse_line(json).unwrap();
        assert!(matches!(entry, RawEntry::QueueOperation));
    }

    #[test]
    fn unknown_type_returns_none() {
        let json = r#"{"type":"unknown_future_type","data":{}}"#;
        assert!(parse_line(json).is_none());
    }

    #[test]
    fn malformed_json_returns_none() {
        assert!(parse_line("not json at all").is_none());
        assert!(parse_line("{broken").is_none());
    }

    #[test]
    fn assistant_null_stop_reason() {
        let json = r#"{"type":"assistant","uuid":"a3","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_03","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"partial"}],"stop_reason":null,"usage":{"input_tokens":3,"output_tokens":2}}}"#;
        let entry = parse_line(json).unwrap();
        match entry {
            RawEntry::Assistant(a) => {
                assert!(a.message.stop_reason.is_none());
            }
            _ => panic!("expected assistant entry"),
        }
    }
}
