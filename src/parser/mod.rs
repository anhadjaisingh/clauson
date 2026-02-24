pub mod raw;
pub mod tool_events;
pub mod transform;

use std::io::BufRead;
use std::path::Path;

use crate::model::session::Session;

/// Parse a JSONL session file into a Session.
pub fn parse_session(path: &Path) -> anyhow::Result<Session> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    let mut transformer = transform::Transformer::new();
    let mut byte_offset: usize = 0;

    for (line_number, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        let byte_length = line.len();
        let line_ref = crate::model::types::RawLineRef {
            line_number: line_number + 1, // 1-indexed
            byte_offset,
            byte_length,
        };

        // Account for line + newline byte
        byte_offset += byte_length + 1;

        if let Some(entry) = raw::parse_line(&line) {
            transformer.process_entry(entry, line_ref);
        }
    }

    let (blocks, provenance, session_id) = transformer.finish();
    let file_path = path.display().to_string();

    Ok(Session::build(blocks, provenance, session_id, file_path))
}
