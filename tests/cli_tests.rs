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

// === stats summary (default) ===

#[test]
fn stats_default_is_summary() {
    // bare `stats` should run summary with all token columns
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tool Name"))
        .stdout(predicate::str::contains("Input"))
        .stdout(predicate::str::contains("Output"))
        .stdout(predicate::str::contains("Cache Create"))
        .stdout(predicate::str::contains("Cache Read"))
        .stdout(predicate::str::contains("Total"));
}

#[test]
fn stats_summary_tokens_by_tool() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "summary"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tool Name"))
        .stdout(predicate::str::contains("% of Total"));
}

#[test]
fn stats_summary_tokens_by_tool_json() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "summary", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn stats_summary_group_by_none() {
    // Replaces old `tokens summary`
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "summary", "--group-by", "none"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Input tokens"))
        .stdout(predicate::str::contains("Total"));
}

#[test]
fn stats_summary_group_by_turn() {
    // Replaces old `tokens by-turn`
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "summary", "--group-by", "turn"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Turn"));
}

#[test]
fn stats_summary_time() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "summary", "--metric", "time"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tool Name"));
}

#[test]
fn stats_summary_time_by_type() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "summary",
            "--metric",
            "time",
            "--group-by",
            "type",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Type"));
}

#[test]
fn stats_summary_tool_calls() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "summary",
            "--metric",
            "tool-calls",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tool Name"))
        .stdout(predicate::str::contains("Count"));
}

#[test]
fn stats_summary_tool_filter() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "summary",
            "--metric",
            "time",
            "--tool",
            "Bash",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Detail"));
}

// === stats summary --token-type ===

#[test]
fn stats_summary_token_type_output() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "summary",
            "--token-type",
            "output",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Output"));
}

#[test]
fn stats_summary_token_type_cache_read() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "summary",
            "--token-type",
            "cache-read",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache Read"));
}

#[test]
fn stats_distribution_token_type() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "distribution",
            "--tool",
            "Bash",
            "--token-type",
            "cache-read",
        ])
        .assert()
        .success();
}

// === stats distribution ===

#[test]
fn stats_distribution_tokens() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "distribution"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tool Name"));
}

#[test]
fn stats_distribution_time() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "distribution",
            "--metric",
            "time",
        ])
        .assert()
        .success();
}

#[test]
fn stats_distribution_json() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "distribution", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn stats_distribution_tool_filter() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "distribution",
            "--tool",
            "Bash",
        ])
        .assert()
        .success();
}

// === stats sample ===

#[test]
fn stats_sample_default() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "sample"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Percentile"))
        .stdout(predicate::str::contains("Turn"));
}

#[test]
fn stats_sample_count() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "sample", "--count", "3"])
        .assert()
        .success();
}

#[test]
fn stats_sample_json() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "sample", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn stats_sample_tool_filter() {
    // Block-level sampling when --tool is specified
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "stats", "sample", "--tool", "Bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Block ID"));
}

#[test]
fn stats_sample_tool_count() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            MEDIUM_FILE,
            "stats",
            "sample",
            "--tool",
            "Bash",
            "--count",
            "2",
        ])
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
fn large_session_stats() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            "testdata/f1cf0635-ee0f-4598-b5f5-1b9d05802a9c.jsonl",
            "stats",
        ])
        .assert()
        .success();
}
