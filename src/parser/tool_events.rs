use std::io::BufRead;
use std::path::Path;

use crate::model::tool_event::ToolEvent;

/// Parse a tool-events JSONL sidecar file into a Vec<ToolEvent>.
pub fn parse_tool_events(path: &Path) -> anyhow::Result<Vec<ToolEvent>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<ToolEvent>(&line) {
            events.push(event);
        }
    }

    Ok(events)
}

/// Derive the sidecar path from a session JSONL path.
/// e.g., "foo.jsonl" -> "foo.tool-events.jsonl"
pub fn sidecar_path(session_path: &Path) -> std::path::PathBuf {
    let stem = session_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let parent = session_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{stem}.tool-events.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sidecar_path_replaces_extension() {
        let p = PathBuf::from("/home/user/.claude/sessions/abc.jsonl");
        assert_eq!(
            sidecar_path(&p),
            PathBuf::from("/home/user/.claude/sessions/abc.tool-events.jsonl")
        );
    }

    #[test]
    fn sidecar_path_no_extension() {
        let p = PathBuf::from("/tmp/session");
        assert_eq!(
            sidecar_path(&p),
            PathBuf::from("/tmp/session.tool-events.jsonl")
        );
    }

    #[test]
    fn parse_fixture() {
        let p = PathBuf::from("testdata/test-session.tool-events.jsonl");
        if !p.exists() {
            return;
        }
        let events = parse_tool_events(&p).unwrap();
        assert!(!events.is_empty());
        assert!(events.len() > 10);
    }
}
