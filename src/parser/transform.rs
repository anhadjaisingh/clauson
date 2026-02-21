use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::model::block::*;
use crate::model::types::*;

use super::raw::*;

/// Pending tool call waiting for its result.
struct PendingToolCall {
    tool_use_id: String,
    tool_name: String,
    input: serde_json::Value,
    assistant_uuid: String,
    timestamp: DateTime<Utc>,
    metadata: EntryMetadata,
}

/// State maintained during transformation.
pub struct Transformer {
    blocks: Vec<Block>,
    provenance: HashMap<NodeId, Vec<RawLineRef>>,
    /// request_id -> index into blocks vec, for merging assistant entries
    assistant_by_request_id: HashMap<String, usize>,
    /// tool_use_id -> pending tool call, waiting for tool_result
    pending_tool_calls: HashMap<String, PendingToolCall>,
    session_id: Option<String>,
}

impl Transformer {
    pub fn new() -> Self {
        Transformer {
            blocks: Vec::new(),
            provenance: HashMap::new(),
            assistant_by_request_id: HashMap::new(),
            pending_tool_calls: HashMap::new(),
            session_id: None,
        }
    }

    /// Process a single raw entry, potentially producing or updating blocks.
    pub fn process_entry(&mut self, entry: RawEntry, line_ref: RawLineRef) {
        match entry {
            RawEntry::User(user) => self.process_user(user, line_ref),
            RawEntry::Assistant(asst) => self.process_assistant(asst, line_ref),
            RawEntry::System(sys) => self.process_system(sys, line_ref),
            RawEntry::Progress | RawEntry::FileHistorySnapshot | RawEntry::QueueOperation => {
                // Skip these entry types
            }
        }
    }

    fn process_user(&mut self, user: RawUserEntry, line_ref: RawLineRef) {
        if self.session_id.is_none() {
            self.session_id.clone_from(&user.common.session_id);
        }

        let metadata = extract_metadata(&user.common);
        let timestamp = user.common.timestamp.unwrap_or_default();
        let uuid = user.common.uuid.unwrap_or_default();

        // Check if this is a tool_result carrier
        match &user.message.content {
            UserContent::Blocks(blocks) => {
                let mut has_tool_result = false;
                let mut has_non_tool_result = false;

                for block in blocks {
                    match block {
                        UserContentBlock::ToolResult(tr) => {
                            has_tool_result = true;
                            // Pair with pending tool call
                            if let Some(mut pending) =
                                self.pending_tool_calls.remove(&tr.tool_use_id)
                            {
                                let tool_block = Block::Tool(ToolBlock {
                                    tool_use_id: pending.tool_use_id,
                                    tool_name: pending.tool_name,
                                    input: std::mem::take(&mut pending.input),
                                    output: tr.content.clone(),
                                    is_error: tr.is_error,
                                    assistant_uuid: pending.assistant_uuid,
                                    result_uuid: Some(uuid.clone()),
                                    timestamp: pending.timestamp,
                                    metadata: pending.metadata,
                                });
                                let id = self.blocks.len();
                                self.blocks.push(tool_block);
                                self.provenance.insert(id, vec![line_ref.clone()]);
                            }
                        }
                        UserContentBlock::Text { .. } => {
                            has_non_tool_result = true;
                        }
                    }
                }

                // If user entry has ONLY tool_results, don't create a UserBlock
                if has_tool_result && !has_non_tool_result {
                    return;
                }

                // Mixed content — create a UserBlock with text content
                let text_content: Vec<String> = blocks
                    .iter()
                    .filter_map(|b| match b {
                        UserContentBlock::Text { text } => text.clone(),
                        _ => None,
                    })
                    .collect();

                let content = if text_content.is_empty() {
                    None
                } else {
                    Some(text_content.join("\n"))
                };

                let user_block = Block::User(UserBlock {
                    uuid: uuid.clone(),
                    parent_uuid: user.common.parent_uuid,
                    timestamp,
                    session_id: user.common.session_id.unwrap_or_default(),
                    content,
                    is_meta: user.common.is_meta,
                    metadata,
                });
                let id = self.blocks.len();
                self.blocks.push(user_block);
                self.provenance.insert(id, vec![line_ref]);
            }
            UserContent::Text(text) => {
                let user_block = Block::User(UserBlock {
                    uuid: uuid.clone(),
                    parent_uuid: user.common.parent_uuid,
                    timestamp,
                    session_id: user.common.session_id.unwrap_or_default(),
                    content: Some(text.clone()),
                    is_meta: user.common.is_meta,
                    metadata,
                });
                let id = self.blocks.len();
                self.blocks.push(user_block);
                self.provenance.insert(id, vec![line_ref]);
            }
        }
    }

    fn process_assistant(&mut self, asst: RawAssistantEntry, line_ref: RawLineRef) {
        if self.session_id.is_none() {
            self.session_id.clone_from(&asst.common.session_id);
        }

        let metadata = extract_metadata(&asst.common);
        let timestamp = asst.common.timestamp.unwrap_or_default();
        let uuid = asst.common.uuid.unwrap_or_default();
        let request_id = asst.request_id.unwrap_or_default();

        // Extract content from this entry
        let mut text_parts = Vec::new();
        let mut thinking_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for content_block in &asst.message.content {
            match content_block {
                AssistantContentBlock::Text { text } => {
                    if let Some(t) = text {
                        if !t.is_empty() {
                            text_parts.push(t.clone());
                        }
                    }
                }
                AssistantContentBlock::Thinking { thinking } => {
                    if let Some(t) = thinking {
                        if !t.is_empty() {
                            thinking_parts.push(t.clone());
                        }
                    }
                }
                AssistantContentBlock::ToolUse(tu) => {
                    tool_calls.push(ToolCall {
                        tool_use_id: tu.id.clone(),
                        tool_name: tu.name.clone(),
                        input: tu.input.clone(),
                    });

                    // Register as pending tool call
                    self.pending_tool_calls.insert(
                        tu.id.clone(),
                        PendingToolCall {
                            tool_use_id: tu.id.clone(),
                            tool_name: tu.name.clone(),
                            input: tu.input.clone(),
                            assistant_uuid: uuid.clone(),
                            timestamp,
                            metadata: metadata.clone(),
                        },
                    );
                }
            }
        }

        let tokens = asst
            .message
            .usage
            .as_ref()
            .map(|u| TokenUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                cache_creation_input_tokens: u.cache_creation_input_tokens,
                cache_read_input_tokens: u.cache_read_input_tokens,
            })
            .unwrap_or_default();

        // Check if we should merge with an existing assistant block (same request_id)
        if !request_id.is_empty() {
            if let Some(&existing_idx) = self.assistant_by_request_id.get(&request_id) {
                // Merge into existing block
                if let Block::Assistant(ref mut existing) = self.blocks[existing_idx] {
                    // Merge text
                    if !text_parts.is_empty() {
                        let new_text = text_parts.join("");
                        match &mut existing.content {
                            Some(content) => content.push_str(&new_text),
                            None => existing.content = Some(new_text),
                        }
                    }

                    // Merge thinking
                    if !thinking_parts.is_empty() {
                        let new_thinking = thinking_parts.join("\n");
                        match &mut existing.thinking {
                            Some(thinking) => {
                                thinking.push('\n');
                                thinking.push_str(&new_thinking);
                            }
                            None => existing.thinking = Some(new_thinking),
                        }
                    }

                    // Merge tool_calls
                    existing.tool_calls.extend(tool_calls);

                    // Merge tokens
                    existing.tokens.merge(&tokens);

                    // Update stop_reason if this entry has one
                    if asst.message.stop_reason.is_some() {
                        existing.stop_reason = asst.message.stop_reason;
                    }
                }

                // Add provenance
                self.provenance
                    .entry(existing_idx)
                    .or_default()
                    .push(line_ref);
                return;
            }
        }

        // Create new assistant block
        let content = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join(""))
        };
        let thinking = if thinking_parts.is_empty() {
            None
        } else {
            Some(thinking_parts.join("\n"))
        };

        let block = Block::Assistant(AssistantBlock {
            uuid: uuid.clone(),
            parent_uuid: asst.common.parent_uuid,
            timestamp,
            session_id: asst.common.session_id.unwrap_or_default(),
            request_id: request_id.clone(),
            model: asst.message.model.unwrap_or_default(),
            content,
            thinking,
            tool_calls,
            stop_reason: asst.message.stop_reason,
            tokens,
            metadata,
        });

        let id = self.blocks.len();
        self.blocks.push(block);
        self.provenance.insert(id, vec![line_ref]);

        if !request_id.is_empty() {
            self.assistant_by_request_id.insert(request_id, id);
        }
    }

    fn process_system(&mut self, sys: RawSystemEntry, line_ref: RawLineRef) {
        if self.session_id.is_none() {
            self.session_id.clone_from(&sys.common.session_id);
        }

        let metadata = extract_metadata(&sys.common);
        let timestamp = sys.common.timestamp.unwrap_or_default();
        let uuid = sys.common.uuid.unwrap_or_default();

        let subtype = match sys.subtype.as_deref() {
            Some("turn_duration") => SystemSubtype::TurnDuration {
                duration_ms: sys.duration_ms.unwrap_or(0),
            },
            Some("stop_hook_summary") => SystemSubtype::StopHookSummary,
            Some("compact_boundary") => {
                let (trigger, pre_tokens) =
                    if let Some(cm) = &sys.compact_metadata {
                        (
                            cm.trigger.clone().unwrap_or_default(),
                            cm.pre_tokens.unwrap_or(0),
                        )
                    } else {
                        (String::new(), 0)
                    };
                SystemSubtype::CompactBoundary {
                    trigger,
                    pre_tokens,
                }
            }
            Some(other) => SystemSubtype::Other {
                subtype: other.to_string(),
            },
            None => SystemSubtype::Other {
                subtype: String::new(),
            },
        };

        let block = Block::System(SystemBlock {
            uuid,
            parent_uuid: sys.common.parent_uuid,
            timestamp,
            session_id: sys.common.session_id.unwrap_or_default(),
            subtype,
            metadata,
        });

        let id = self.blocks.len();
        self.blocks.push(block);
        self.provenance.insert(id, vec![line_ref]);
    }

    /// Finalize: flush any unpaired tool calls as ToolBlocks without output.
    pub fn finish(mut self) -> (Vec<Block>, HashMap<NodeId, Vec<RawLineRef>>, Option<String>) {
        // Flush unpaired pending tool calls
        for (_id, pending) in self.pending_tool_calls.drain() {
            let block = Block::Tool(ToolBlock {
                tool_use_id: pending.tool_use_id,
                tool_name: pending.tool_name,
                input: pending.input,
                output: None,
                is_error: false,
                assistant_uuid: pending.assistant_uuid,
                result_uuid: None,
                timestamp: pending.timestamp,
                metadata: pending.metadata,
            });
            self.blocks.push(block);
        }

        (self.blocks, self.provenance, self.session_id)
    }
}

fn extract_metadata(common: &CommonFields) -> EntryMetadata {
    EntryMetadata {
        version: common.version.clone(),
        cwd: common.cwd.clone(),
        git_branch: common.git_branch.clone(),
        slug: common.slug.clone(),
        is_sidechain: common.is_sidechain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_line_ref(line: usize) -> RawLineRef {
        RawLineRef {
            line_number: line,
            byte_offset: 0,
            byte_length: 0,
        }
    }

    fn transform_single(json: &str) -> Vec<Block> {
        let mut t = Transformer::new();
        let entry: RawEntry = serde_json::from_str(json).unwrap();
        t.process_entry(entry, make_line_ref(1));
        let (blocks, _, _) = t.finish();
        blocks
    }

    fn transform_sequence(jsons: &[&str]) -> Vec<Block> {
        let mut t = Transformer::new();
        for (i, json) in jsons.iter().enumerate() {
            let entry: RawEntry = serde_json::from_str(json).unwrap();
            t.process_entry(entry, make_line_ref(i + 1));
        }
        let (blocks, _, _) = t.finish();
        blocks
    }

    #[test]
    fn transform_user_text_message() {
        let json = r#"{"type":"user","uuid":"u1","parentUuid":null,"timestamp":"2026-02-18T14:46:16.829Z","sessionId":"sess1","isSidechain":false,"message":{"role":"user","content":"Hello world"}}"#;
        let blocks = transform_single(json);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::User(u) => {
                assert_eq!(u.uuid, "u1");
                assert_eq!(u.content.as_deref(), Some("Hello world"));
                assert!(!u.is_meta);
            }
            _ => panic!("expected UserBlock"),
        }
    }

    #[test]
    fn transform_user_meta() {
        let json = r#"{"type":"user","uuid":"u1","parentUuid":null,"timestamp":"2026-02-18T14:46:16.829Z","sessionId":"sess1","isSidechain":false,"isMeta":true,"message":{"role":"user","content":"meta content"}}"#;
        let blocks = transform_single(json);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::User(u) => assert!(u.is_meta),
            _ => panic!("expected UserBlock"),
        }
    }

    #[test]
    fn transform_assistant_text() {
        let json = r#"{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"Hello!"}],"stop_reason":"end_turn","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":200,"cache_read_input_tokens":300}}}"#;
        let blocks = transform_single(json);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Assistant(a) => {
                assert_eq!(a.content.as_deref(), Some("Hello!"));
                assert_eq!(a.tokens.input_tokens, 100);
                assert_eq!(a.tokens.output_tokens, 50);
                assert_eq!(a.model, "claude-opus-4-6");
                assert_eq!(a.stop_reason.as_deref(), Some("end_turn"));
            }
            _ => panic!("expected AssistantBlock"),
        }
    }

    #[test]
    fn transform_assistant_with_thinking() {
        let json = r#"{"type":"assistant","uuid":"a1","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"thinking","thinking":"Let me think..."},{"type":"text","text":"Answer."}],"stop_reason":"end_turn"}}"#;
        let blocks = transform_single(json);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Assistant(a) => {
                assert_eq!(a.thinking.as_deref(), Some("Let me think..."));
                assert_eq!(a.content.as_deref(), Some("Answer."));
            }
            _ => panic!("expected AssistantBlock"),
        }
    }

    #[test]
    fn transform_assistant_with_tool_calls() {
        let json = r#"{"type":"assistant","uuid":"a1","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"Let me read that."},{"type":"tool_use","id":"toolu_01","name":"Read","input":{"file_path":"/tmp/test"}}],"stop_reason":"tool_use"}}"#;
        let blocks = transform_single(json);
        // Assistant block + unpaired tool call
        assert_eq!(blocks.len(), 2);
        match &blocks[0] {
            Block::Assistant(a) => {
                assert_eq!(a.tool_calls.len(), 1);
                assert_eq!(a.tool_calls[0].tool_name, "Read");
                assert_eq!(a.tool_calls[0].tool_use_id, "toolu_01");
            }
            _ => panic!("expected AssistantBlock"),
        }
        // Unpaired tool call becomes ToolBlock with no output
        match &blocks[1] {
            Block::Tool(t) => {
                assert_eq!(t.tool_name, "Read");
                assert!(t.output.is_none());
            }
            _ => panic!("expected ToolBlock"),
        }
    }

    #[test]
    fn transform_tool_result_pairs_with_tool_use() {
        let asst_json = r#"{"type":"assistant","uuid":"a1","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"tool_use","id":"toolu_01","name":"Read","input":{"file_path":"/tmp/test"}}],"stop_reason":"tool_use"}}"#;
        let user_json = r#"{"type":"user","uuid":"u2","parentUuid":"a1","timestamp":"2026-02-18T14:46:21.000Z","sessionId":"sess1","isSidechain":false,"message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_01","content":"file contents","is_error":false}]}}"#;

        let blocks = transform_sequence(&[asst_json, user_json]);
        // Should have: AssistantBlock + ToolBlock (paired)
        assert_eq!(blocks.len(), 2);
        match &blocks[1] {
            Block::Tool(t) => {
                assert_eq!(t.tool_name, "Read");
                assert_eq!(
                    t.output.as_ref().unwrap(),
                    &serde_json::json!("file contents")
                );
                assert!(!t.is_error);
                assert_eq!(t.result_uuid.as_deref(), Some("u2"));
            }
            _ => panic!("expected ToolBlock"),
        }
        // Tool result carrier should NOT produce a UserBlock
        assert!(
            !blocks.iter().any(|b| matches!(b, Block::User(_))),
            "tool_result-only user entries should not produce UserBlocks"
        );
    }

    #[test]
    fn transform_assistant_merge_by_request_id() {
        let first = r#"{"type":"assistant","uuid":"a1","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"First part "}],"stop_reason":null,"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        let second = r#"{"type":"assistant","uuid":"a2","timestamp":"2026-02-18T14:46:21.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"second part."}],"stop_reason":"end_turn","usage":{"input_tokens":20,"output_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;

        let blocks = transform_sequence(&[first, second]);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Assistant(a) => {
                assert_eq!(a.content.as_deref(), Some("First part second part."));
                assert_eq!(a.tokens.input_tokens, 30);
                assert_eq!(a.tokens.output_tokens, 15);
                assert_eq!(a.stop_reason.as_deref(), Some("end_turn"));
            }
            _ => panic!("expected AssistantBlock"),
        }
    }

    #[test]
    fn transform_system_turn_duration() {
        let json = r#"{"type":"system","subtype":"turn_duration","durationMs":12345,"timestamp":"2026-02-18T14:48:50.406Z","uuid":"s1","sessionId":"sess1","isSidechain":false}"#;
        let blocks = transform_single(json);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::System(s) => match &s.subtype {
                SystemSubtype::TurnDuration { duration_ms } => {
                    assert_eq!(*duration_ms, 12345);
                }
                _ => panic!("expected TurnDuration"),
            },
            _ => panic!("expected SystemBlock"),
        }
    }

    #[test]
    fn transform_system_compact_boundary() {
        let json = r#"{"type":"system","subtype":"compact_boundary","timestamp":"2026-02-18T14:18:01.734Z","uuid":"s2","sessionId":"sess1","isSidechain":false,"compactMetadata":{"trigger":"auto","preTokens":169054}}"#;
        let blocks = transform_single(json);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::System(s) => match &s.subtype {
                SystemSubtype::CompactBoundary {
                    trigger,
                    pre_tokens,
                } => {
                    assert_eq!(trigger, "auto");
                    assert_eq!(*pre_tokens, 169054);
                }
                _ => panic!("expected CompactBoundary"),
            },
            _ => panic!("expected SystemBlock"),
        }
    }

    #[test]
    fn transform_skips_progress_and_snapshots() {
        let progress = r#"{"type":"progress","data":{"type":"hook_progress"},"uuid":"p1","timestamp":"2026-02-18T14:34:31.267Z"}"#;
        let snapshot = r#"{"type":"file-history-snapshot","messageId":"m1","snapshot":{}}"#;
        let blocks = transform_sequence(&[progress, snapshot]);
        assert!(blocks.is_empty());
    }

    #[test]
    fn transform_token_merging() {
        let first = r#"{"type":"assistant","uuid":"a1","timestamp":"2026-02-18T14:46:20.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"a"}],"stop_reason":null,"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":100,"cache_read_input_tokens":200}}}"#;
        let second = r#"{"type":"assistant","uuid":"a2","timestamp":"2026-02-18T14:46:21.572Z","sessionId":"sess1","isSidechain":false,"requestId":"req_01","message":{"model":"claude-opus-4-6","role":"assistant","content":[{"type":"text","text":"b"}],"stop_reason":"end_turn","usage":{"input_tokens":20,"output_tokens":10,"cache_creation_input_tokens":50,"cache_read_input_tokens":100}}}"#;

        let blocks = transform_sequence(&[first, second]);
        match &blocks[0] {
            Block::Assistant(a) => {
                assert_eq!(a.tokens.input_tokens, 30);
                assert_eq!(a.tokens.output_tokens, 15);
                assert_eq!(a.tokens.cache_creation_input_tokens, 150);
                assert_eq!(a.tokens.cache_read_input_tokens, 300);
            }
            _ => panic!("expected AssistantBlock"),
        }
    }
}
