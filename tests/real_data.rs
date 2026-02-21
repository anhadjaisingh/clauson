use clauson::model::block::{Block, BlockInfo};
use clauson::model::types::BlockType;
use clauson::parser::parse_session;
use std::path::Path;

#[test]
fn parse_small_session() {
    let session =
        parse_session(Path::new("testdata/421d2e3a-f3d7-4c79-9e04-459471305d6f.jsonl")).unwrap();
    assert!(!session.blocks.is_empty(), "should have some blocks");
}

#[test]
fn parse_medium_session() {
    let session =
        parse_session(Path::new("testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl")).unwrap();

    assert!(!session.blocks_of_type(BlockType::Assistant).is_empty());
    assert!(!session.blocks_of_type(BlockType::User).is_empty());
    assert!(!session.blocks_of_type(BlockType::Tool).is_empty());
    assert!(!session.blocks_of_type(BlockType::System).is_empty());

    // Every tool block should have a non-empty tool_name
    for &id in session.blocks_of_type(BlockType::Tool) {
        if let Block::Tool(t) = session.block(id) {
            assert!(!t.tool_name.is_empty(), "tool block should have a name");
        }
    }
}

#[test]
fn parse_medium_session_2() {
    let session =
        parse_session(Path::new("testdata/058c4c27-07c1-4f93-86c1-317a4faa9803.jsonl")).unwrap();
    assert!(!session.blocks.is_empty());
}

#[test]
fn parse_medium_session_3() {
    let session =
        parse_session(Path::new("testdata/6577be84-6784-4198-b13e-25baaaa2e1d2.jsonl")).unwrap();
    assert!(!session.blocks.is_empty());
}

#[test]
fn parse_medium_session_4() {
    let session =
        parse_session(Path::new("testdata/f4f38a6b-a385-4732-916a-0312b9455d5f.jsonl")).unwrap();
    assert!(!session.blocks.is_empty());
}

#[test]
fn parse_snapshot_only_file_2() {
    // This file also contains only file-history-snapshot entries
    let session =
        parse_session(Path::new("testdata/7007e87f-c87d-47d8-b05b-21d903656132.jsonl")).unwrap();
    assert_eq!(
        session.blocks.len(),
        0,
        "snapshot-only file should produce no blocks"
    );
}

#[test]
fn parse_large_session() {
    let session =
        parse_session(Path::new("testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl")).unwrap();
    assert!(
        session.blocks.len() > 100,
        "large session should have many blocks, got {}",
        session.blocks.len()
    );
    assert!(!session.blocks_of_type(BlockType::System).is_empty());
}

#[test]
fn parse_snapshot_only_file() {
    let session =
        parse_session(Path::new("testdata/15272057-5296-4b76-a119-e4af992a70e0.jsonl")).unwrap();
    assert_eq!(
        session.blocks.len(),
        0,
        "snapshot-only file should produce no blocks"
    );
}

#[test]
fn provenance_tracks_line_numbers() {
    let session =
        parse_session(Path::new("testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl")).unwrap();
    for id in 0..session.blocks.len() {
        let prov = session.provenance.get(&id);
        assert!(prov.is_some(), "block {id} should have provenance");
        assert!(
            !prov.unwrap().is_empty(),
            "block {id} should have at least one line ref"
        );
        for line_ref in prov.unwrap() {
            assert!(line_ref.line_number >= 1);
        }
    }
}

#[test]
fn assistant_request_id_merging() {
    let session =
        parse_session(Path::new("testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl")).unwrap();
    let assistant_count = session.blocks_of_type(BlockType::Assistant).len();
    // Raw file has 58 assistant entries, merging by requestId should produce fewer blocks
    assert!(
        assistant_count < 58,
        "merged assistant count ({assistant_count}) should be less than raw entry count (58)"
    );
    assert!(assistant_count > 0, "should have some assistant blocks");
}

#[test]
fn turns_detected_in_medium_session() {
    let session =
        parse_session(Path::new("testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl")).unwrap();
    let turns = session.turns();
    assert!(!turns.is_empty(), "should detect at least one turn");
    for turn in &turns {
        assert_eq!(
            session.block(turn.user_block).block_type(),
            BlockType::User,
            "each turn should start with a user block"
        );
    }
}
