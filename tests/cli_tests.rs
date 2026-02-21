use assert_cmd::Command;
use predicates::prelude::*;

const MEDIUM_FILE: &str = "testdata/e998eca1-e455-49e8-86fb-bbfbc88f74f0.jsonl";

// === blocks list ===

#[test]
fn blocks_list_runs() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "blocks", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("user"))
        .stdout(predicate::str::contains("assistant"));
}

#[test]
fn blocks_list_filter_by_type() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "blocks", "list", "--type", "tool"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tool"));
}

#[test]
fn blocks_list_json_valid() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "blocks", "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn blocks_default_is_list() {
    let with_list = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "blocks", "list"])
        .output()
        .unwrap();
    let without_list = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "blocks"])
        .output()
        .unwrap();
    assert_eq!(with_list.stdout, without_list.stdout);
}

// === blocks count ===

#[test]
fn blocks_count_by_type() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "blocks", "count"])
        .assert()
        .success()
        .stdout(predicate::str::contains("user"))
        .stdout(predicate::str::contains("assistant"));
}

#[test]
fn blocks_count_by_tool() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "blocks", "count", "--group-by", "tool"])
        .assert()
        .success();
}

#[test]
fn blocks_count_json() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "blocks", "count", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let _: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
}

// === tools list ===

#[test]
fn tools_list_shows_unique_tools() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tools", "list"])
        .assert()
        .success();
}

#[test]
fn tools_list_json() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tools", "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn tools_default_is_list() {
    // Use --json for deterministic output (HashMap order can vary)
    let with_list = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tools", "list", "--json"])
        .output()
        .unwrap();
    let without_list = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tools", "--json"])
        .output()
        .unwrap();
    assert_eq!(with_list.stdout, without_list.stdout);
}

// === tokens ===

#[test]
fn tokens_summary() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tokens"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Input tokens"));
}

#[test]
fn tokens_by_turn() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tokens", "by-turn"])
        .assert()
        .success();
}

#[test]
fn tokens_summary_json() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tokens", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let _: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
}

// === turns ===

#[test]
fn turns_list() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "turns", "list"])
        .assert()
        .success();
}

#[test]
fn turns_list_json() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "turns", "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn turns_default_is_list() {
    let with_list = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "turns", "list"])
        .output()
        .unwrap();
    let without_list = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "turns"])
        .output()
        .unwrap();
    assert_eq!(with_list.stdout, without_list.stdout);
}

#[test]
fn turns_show() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "turns", "show", "1"])
        .assert()
        .success();
}

// === edge cases ===

#[test]
fn large_session_blocks_count() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            "testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl",
            "blocks",
            "count",
        ])
        .assert()
        .success();
}

#[test]
fn large_session_tools() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            "testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl",
            "tools",
        ])
        .assert()
        .success();
}

#[test]
fn large_session_tokens() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            "testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl",
            "tokens",
        ])
        .assert()
        .success();
}
