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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1700000000 + secs, 0).unwrap()
    }

    fn make_event(kind: ToolEventKind, tool: &str, id: &str, t: DateTime<Utc>) -> ToolEvent {
        ToolEvent {
            event: kind,
            tool_name: tool.to_string(),
            tool_use_id: id.to_string(),
            tool_input: serde_json::json!({}),
            session_id: None,
            permission_mode: None,
            timestamp: t,
            permission_suggestions: None,
        }
    }

    // --- ToolEventKind display ---

    #[test]
    fn tool_event_kind_display() {
        assert_eq!(ToolEventKind::PreToolUse.to_string(), "PreToolUse");
        assert_eq!(ToolEventKind::PermissionRequest.to_string(), "PermissionRequest");
        assert_eq!(ToolEventKind::PostToolUse.to_string(), "PostToolUse");
        assert_eq!(ToolEventKind::PostToolUseFailure.to_string(), "PostToolUseFailure");
    }

    // --- Lifecycle classification ---

    #[test]
    fn lifecycle_auto_approved() {
        let lc = ToolCallLifecycle {
            tool_use_id: "t1".into(),
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({}),
            pre_tool_use: Some(ts(0)),
            permission_request: None,
            completion: Some(ts(1)),
            succeeded: true,
        };
        assert!(!lc.was_prompted());
        assert!(!lc.was_denied());
        assert_eq!(lc.status_label(), "auto-approved");
    }

    #[test]
    fn lifecycle_prompted_approved() {
        let lc = ToolCallLifecycle {
            tool_use_id: "t2".into(),
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({}),
            pre_tool_use: Some(ts(0)),
            permission_request: Some(ts(1)),
            completion: Some(ts(5)),
            succeeded: true,
        };
        assert!(lc.was_prompted());
        assert!(!lc.was_denied());
        assert_eq!(lc.status_label(), "prompted->approved");
    }

    #[test]
    fn lifecycle_prompted_denied() {
        let lc = ToolCallLifecycle {
            tool_use_id: "t3".into(),
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({}),
            pre_tool_use: Some(ts(0)),
            permission_request: Some(ts(1)),
            completion: Some(ts(2)),
            succeeded: false,
        };
        assert!(lc.was_prompted());
        assert!(lc.was_denied());
        assert_eq!(lc.status_label(), "prompted->denied");
    }

    #[test]
    fn lifecycle_no_completion() {
        // PermissionRequest but no PostToolUse/PostToolUseFailure (session ended)
        let lc = ToolCallLifecycle {
            tool_use_id: "t4".into(),
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({}),
            pre_tool_use: Some(ts(0)),
            permission_request: Some(ts(1)),
            completion: None,
            succeeded: false,
        };
        assert!(lc.was_prompted());
        assert!(lc.was_denied());
        assert_eq!(lc.status_label(), "prompted->denied");
    }

    #[test]
    fn lifecycle_permission_wait_secs() {
        let lc = ToolCallLifecycle {
            tool_use_id: "t5".into(),
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({}),
            pre_tool_use: Some(ts(0)),
            permission_request: Some(ts(10)),
            completion: Some(ts(14)),
            succeeded: true,
        };
        let wait = lc.permission_wait_secs().unwrap();
        assert!((wait - 4.0).abs() < 0.001);
    }

    #[test]
    fn lifecycle_no_wait_when_auto_approved() {
        let lc = ToolCallLifecycle {
            tool_use_id: "t6".into(),
            tool_name: "Read".into(),
            tool_input: serde_json::json!({}),
            pre_tool_use: Some(ts(0)),
            permission_request: None,
            completion: Some(ts(1)),
            succeeded: true,
        };
        assert!(lc.permission_wait_secs().is_none());
    }

    // --- build_lifecycles ---

    #[test]
    fn build_lifecycles_groups_by_tool_use_id() {
        let events = vec![
            make_event(ToolEventKind::PreToolUse, "Bash", "t1", ts(0)),
            make_event(ToolEventKind::PermissionRequest, "Bash", "t1", ts(1)),
            make_event(ToolEventKind::PostToolUse, "Bash", "t1", ts(5)),
        ];
        let lifecycles = build_lifecycles(&events);
        assert_eq!(lifecycles.len(), 1);
        assert!(lifecycles[0].pre_tool_use.is_some());
        assert!(lifecycles[0].permission_request.is_some());
        assert!(lifecycles[0].succeeded);
    }

    #[test]
    fn build_lifecycles_preserves_order() {
        let events = vec![
            make_event(ToolEventKind::PreToolUse, "Read", "t2", ts(0)),
            make_event(ToolEventKind::PreToolUse, "Bash", "t1", ts(1)),
            make_event(ToolEventKind::PostToolUse, "Read", "t2", ts(2)),
            make_event(ToolEventKind::PostToolUse, "Bash", "t1", ts(3)),
        ];
        let lifecycles = build_lifecycles(&events);
        assert_eq!(lifecycles.len(), 2);
        assert_eq!(lifecycles[0].tool_use_id, "t2");
        assert_eq!(lifecycles[1].tool_use_id, "t1");
    }

    #[test]
    fn build_lifecycles_empty_input() {
        let lifecycles = build_lifecycles(&[]);
        assert!(lifecycles.is_empty());
    }
}
