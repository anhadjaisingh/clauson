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

// === tool-events ===
//
// Test fixture: testdata/test-session.jsonl + testdata/test-session.tool-events.jsonl
// Fixture contains: 10 calls (5 Bash, 3 Read, 2 Write), 5 prompted, 1 denied
//
// See docs/plans/2026-02-25-tool-events-test-plan.md for what each test protects.

const TOOL_EVENTS_SESSION: &str = "testdata/test-session.jsonl";

#[test]
fn tool_events_missing_sidecar() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([MEDIUM_FILE, "tool-events"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No tool events file found"));
}

// --- summary ---

#[test]
fn tool_events_summary_runs() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tool"))
        .stdout(predicate::str::contains("Calls"))
        .stdout(predicate::str::contains("Prompted"))
        .stdout(predicate::str::contains("Denied"));
}

#[test]
fn tool_events_summary_json_valid() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "summary", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn tool_events_summary_counts_correct() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "summary", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    // Last entry is Total
    let total = parsed.last().unwrap();
    assert_eq!(total["calls"], 10);
    assert_eq!(total["prompted"], 5);
    assert_eq!(total["denied"], 1);
}

#[test]
fn tool_events_default_is_summary() {
    let with = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "summary", "--json"])
        .output()
        .unwrap();
    let without = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "--json"])
        .output()
        .unwrap();
    assert_eq!(with.stdout, without.stdout);
}

// --- list ---

#[test]
fn tool_events_list_runs() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PreToolUse"))
        .stdout(predicate::str::contains("Bash"));
}

#[test]
fn tool_events_list_filter_tool() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "list",
            "--tool",
            "Read",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for entry in &parsed {
        assert_eq!(entry["tool_name"], "Read");
    }
}

#[test]
fn tool_events_list_filter_event() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "list",
            "--event",
            "PermissionRequest",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for entry in &parsed {
        assert_eq!(entry["event"], "PermissionRequest");
    }
    // 3 Bash + 2 Write = 5 PermissionRequest events
    assert_eq!(parsed.len(), 5);
}

// --- timeline ---

#[test]
fn tool_events_timeline_runs() {
    Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "timeline"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auto-approved"))
        .stdout(predicate::str::contains("prompted->approved"));
}

#[test]
fn tool_events_timeline_filter_tool() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "timeline",
            "--tool",
            "Bash",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed.len(), 5); // 5 Bash calls
    for entry in &parsed {
        assert_eq!(entry["tool_name"], "Bash");
    }
}

#[test]
fn tool_events_timeline_shows_denied() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "timeline", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    let denied: Vec<_> = parsed
        .iter()
        .filter(|e| e["status"] == "prompted->denied")
        .collect();
    assert_eq!(denied.len(), 1);
}

#[test]
fn tool_events_timeline_json_has_wait() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "timeline", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    // Find a prompted->approved entry - it should have wait_secs > 0
    let prompted = parsed
        .iter()
        .find(|e| e["status"] == "prompted->approved")
        .unwrap();
    assert!(prompted["wait_secs"].as_f64().unwrap() > 0.0);
}

// === tool-events real-format ===
//
// Test fixture: testdata/test-session-real-format.jsonl + .tool-events.jsonl
// Uses real production format: tool_use_id is null on PermissionRequest,
// permission_suggestions contains objects (not strings).
//
// Fixture contains: 8 calls (4 Bash, 2 Read, 1 Write, 1 WebFetch)
//   - 2 auto-approved (Bash ls, Read /tmp/foo.rs)
//   - 5 prompted->approved (Bash cargo test, Read /etc/passwd, Write, Bash npm install, Bash git push)
//   - 1 prompted->denied (WebFetch)
//   Total: 6 prompted, 1 denied

const REAL_FORMAT_SESSION: &str = "testdata/test-session-real-format.jsonl";

#[test]
fn tool_events_real_format_parses() {
    // Fixture should parse without errors and produce correct total count
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([REAL_FORMAT_SESSION, "tool-events", "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    // 8 PreToolUse + 6 PermissionRequest + 7 PostToolUse + 1 PostToolUseFailure = 22
    assert_eq!(parsed.len(), 22);
    // No warnings on stderr (all lines parse successfully)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("warning: skipped"),
        "unexpected parse warnings: {stderr}"
    );
}

#[test]
fn tool_events_real_format_summary_counts() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([REAL_FORMAT_SESSION, "tool-events", "summary", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    let total = parsed.last().unwrap();
    assert_eq!(total["calls"], 8);
    assert_eq!(total["prompted"], 6);
    assert_eq!(total["denied"], 1);
}

#[test]
fn tool_events_real_format_timeline_statuses() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([REAL_FORMAT_SESSION, "tool-events", "timeline", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed.len(), 8);
    let auto: Vec<_> = parsed
        .iter()
        .filter(|e| e["status"] == "auto-approved")
        .collect();
    let approved: Vec<_> = parsed
        .iter()
        .filter(|e| e["status"] == "prompted->approved")
        .collect();
    let denied: Vec<_> = parsed
        .iter()
        .filter(|e| e["status"] == "prompted->denied")
        .collect();
    assert_eq!(auto.len(), 2);
    assert_eq!(approved.len(), 5);
    assert_eq!(denied.len(), 1);
}

#[test]
fn tool_events_real_format_timeline_wait_times() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([REAL_FORMAT_SESSION, "tool-events", "timeline", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    // All prompted entries should have wait_secs > 0
    for entry in &parsed {
        if entry["status"] == "prompted->approved" || entry["status"] == "prompted->denied" {
            assert!(
                entry["wait_secs"].as_f64().unwrap() > 0.0,
                "prompted entry should have wait_secs > 0: {entry}"
            );
        }
    }
}

#[test]
fn tool_events_real_format_list_permission_request() {
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            REAL_FORMAT_SESSION,
            "tool-events",
            "list",
            "--event",
            "PermissionRequest",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed.len(), 6);
    for entry in &parsed {
        assert_eq!(entry["event"], "PermissionRequest");
    }
}

// --- summary wait time columns ---

#[test]
fn tool_events_summary_has_wait() {
    // Table output should contain Wait and Wait% headers
    Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "summary"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Wait"))
        .stdout(predicate::str::contains("Wait%"));
}

#[test]
fn tool_events_summary_json_has_wait() {
    // JSON entries should have wait_secs field
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([TOOL_EVENTS_SESSION, "tool-events", "summary", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    for entry in &parsed {
        assert!(
            entry.get("wait_secs").is_some(),
            "entry missing wait_secs: {entry}"
        );
    }
    // Bash has prompted calls so should have wait > 0
    let bash = parsed.iter().find(|e| e["tool_name"] == "Bash").unwrap();
    assert!(bash["wait_secs"].as_f64().unwrap() > 0.0);
    // Read has no prompted calls so wait should be 0
    let read = parsed.iter().find(|e| e["tool_name"] == "Read").unwrap();
    assert_eq!(read["wait_secs"].as_f64().unwrap(), 0.0);
}

// --- summary --tool drill-down ---

#[test]
fn tool_events_summary_drilldown_runs() {
    // --tool Bash should show Detail column instead of Tool column
    Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "summary",
            "--tool",
            "Bash",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Detail"))
        .stdout(predicate::str::contains("Calls"))
        .stdout(predicate::str::contains("Prompted"));
}

#[test]
fn tool_events_summary_drilldown_json() {
    // --tool Bash --json should have detail field with correct aggregate counts
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            TOOL_EVENTS_SESSION,
            "tool-events",
            "summary",
            "--tool",
            "Bash",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    // Each entry should have detail field
    for entry in &parsed {
        assert!(
            entry.get("detail").is_some(),
            "entry missing detail: {entry}"
        );
    }
    // Total row should match fixture: 5 Bash calls, 3 prompted, 1 denied
    let total = parsed.last().unwrap();
    assert_eq!(total["detail"], "Total");
    assert_eq!(total["calls"], 5);
    assert_eq!(total["prompted"], 3);
    assert_eq!(total["denied"], 1);
}

#[test]
fn tool_events_real_format_drilldown() {
    // --tool Bash --json on real-format fixture should show prompted bash details
    let output = Command::cargo_bin("clauson")
        .unwrap()
        .args([
            REAL_FORMAT_SESSION,
            "tool-events",
            "summary",
            "--tool",
            "Bash",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    // Real-format has 4 Bash calls, 3 prompted, 0 denied
    let total = parsed.last().unwrap();
    assert_eq!(total["detail"], "Total");
    assert_eq!(total["calls"], 4);
    assert_eq!(total["prompted"], 3);
    assert_eq!(total["denied"], 0);
    // Should include specific bash commands as details
    let details: Vec<_> = parsed
        .iter()
        .filter(|e| e["detail"] != "Total")
        .map(|e| e["detail"].as_str().unwrap().to_string())
        .collect();
    assert!(details.contains(&"cargo test".to_string()));
}
