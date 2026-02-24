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
