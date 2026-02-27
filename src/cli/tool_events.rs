use anyhow::Result;
use clap::Subcommand;
use std::collections::HashMap;
use std::path::Path;

use clauson::model::tool_event::{build_lifecycles, ToolCallLifecycle};
use clauson::parser::tool_events::{parse_tool_events, sidecar_path};

use super::output;

#[derive(Subcommand)]
pub enum ToolEventsAction {
    /// Aggregated permission stats per tool (default)
    Summary {
        /// Drill down into a specific tool's details
        #[arg(long)]
        tool: Option<String>,
    },
    /// Chronological event stream
    List {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
        /// Filter by event type
        #[arg(long)]
        event: Option<String>,
    },
    /// Per-tool-call lifecycle with permission wait times
    Timeline {
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
    },
}

pub fn run(session_path: &Path, action: Option<&ToolEventsAction>, json: bool) -> Result<()> {
    let sidecar = sidecar_path(session_path);
    if !sidecar.exists() {
        eprintln!(
            "No tool events file found at: {}\n\
             Install the clauson-hooks plugin to collect tool event data:\n\
             /plugin add <path-to-clauson>/plugin",
            sidecar.display()
        );
        std::process::exit(1);
    }

    let events = parse_tool_events(&sidecar)?;
    if events.is_empty() {
        println!("No tool events recorded.");
        return Ok(());
    }

    match action {
        None => run_summary(&events, None, json),
        Some(ToolEventsAction::Summary { tool }) => run_summary(&events, tool.as_deref(), json),
        Some(ToolEventsAction::List { tool, event }) => {
            run_list(&events, tool.as_deref(), event.as_deref(), json)
        }
        Some(ToolEventsAction::Timeline { tool }) => {
            let lifecycles = build_lifecycles(&events);
            run_timeline(&lifecycles, tool.as_deref(), json)
        }
    }
}

fn run_summary(
    events: &[clauson::model::tool_event::ToolEvent],
    tool_filter: Option<&str>,
    json: bool,
) -> Result<()> {
    let lifecycles = build_lifecycles(events);

    if let Some(tool_name) = tool_filter {
        return run_summary_tool_detail(&lifecycles, tool_name, json);
    }

    struct ToolStats {
        calls: usize,
        prompted: usize,
        denied: usize,
        wait_secs: f64,
    }

    let mut stats: HashMap<String, ToolStats> = HashMap::new();
    for lc in &lifecycles {
        let entry = stats.entry(lc.tool_name.clone()).or_insert(ToolStats {
            calls: 0,
            prompted: 0,
            denied: 0,
            wait_secs: 0.0,
        });
        entry.calls += 1;
        if lc.was_prompted() {
            entry.prompted += 1;
        }
        if lc.was_denied() {
            entry.denied += 1;
        }
        if let Some(w) = lc.permission_wait_secs() {
            entry.wait_secs += w;
        }
    }

    let mut entries: Vec<_> = stats.into_iter().collect();
    entries.sort_by(|a, b| b.1.calls.cmp(&a.1.calls));

    let total_calls: usize = entries.iter().map(|(_, s)| s.calls).sum();
    let total_prompted: usize = entries.iter().map(|(_, s)| s.prompted).sum();
    let total_denied: usize = entries.iter().map(|(_, s)| s.denied).sum();
    let total_wait: f64 = entries.iter().map(|(_, s)| s.wait_secs).sum();

    if json {
        let mut json_entries: Vec<_> = entries
            .iter()
            .map(|(name, s)| {
                serde_json::json!({
                    "tool_name": name,
                    "calls": s.calls,
                    "prompted": s.prompted,
                    "prompt_percent": format!("{:.1}", pct(s.prompted, s.calls)),
                    "denied": s.denied,
                    "deny_percent": format!("{:.1}", pct(s.denied, s.calls)),
                    "wait_secs": round2(s.wait_secs),
                    "wait_percent": format!("{:.1}", pct_f64(s.wait_secs, total_wait)),
                })
            })
            .collect();
        json_entries.push(serde_json::json!({
            "tool_name": "Total",
            "calls": total_calls,
            "prompted": total_prompted,
            "prompt_percent": format!("{:.1}", pct(total_prompted, total_calls)),
            "denied": total_denied,
            "deny_percent": format!("{:.1}", pct(total_denied, total_calls)),
            "wait_secs": round2(total_wait),
            "wait_percent": "100.0",
        }));
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Tool"),
            output::Column::right("Calls"),
            output::Column::right("Prompted"),
            output::Column::right("Prompt%"),
            output::Column::right("Denied"),
            output::Column::right("Deny%"),
            output::Column::right("Wait"),
            output::Column::right("Wait%"),
        ]);
        for (name, s) in &entries {
            table.add_row(vec![
                name.clone(),
                s.calls.to_string(),
                s.prompted.to_string(),
                format!("{:.1}%", pct(s.prompted, s.calls)),
                s.denied.to_string(),
                format!("{:.1}%", pct(s.denied, s.calls)),
                format_wait(s.wait_secs),
                format!("{:.1}%", pct_f64(s.wait_secs, total_wait)),
            ]);
        }
        table.print_with_total(&format!(
            "Total: {} calls, {} prompted ({:.1}%), {} denied ({:.1}%), {} wait",
            total_calls,
            total_prompted,
            pct(total_prompted, total_calls),
            total_denied,
            pct(total_denied, total_calls),
            format_wait(total_wait),
        ));
    }

    Ok(())
}

fn run_summary_tool_detail(
    lifecycles: &[ToolCallLifecycle],
    tool_name: &str,
    json: bool,
) -> Result<()> {
    struct DetailStats {
        calls: usize,
        prompted: usize,
        denied: usize,
        wait_secs: f64,
    }

    let mut buckets: HashMap<String, DetailStats> = HashMap::new();
    for lc in lifecycles {
        if lc.tool_name != tool_name {
            continue;
        }
        let detail = extract_tool_detail(&lc.tool_name, &lc.tool_input);
        let entry = buckets.entry(detail).or_insert(DetailStats {
            calls: 0,
            prompted: 0,
            denied: 0,
            wait_secs: 0.0,
        });
        entry.calls += 1;
        if lc.was_prompted() {
            entry.prompted += 1;
        }
        if lc.was_denied() {
            entry.denied += 1;
        }
        if let Some(w) = lc.permission_wait_secs() {
            entry.wait_secs += w;
        }
    }

    let mut entries: Vec<_> = buckets.into_iter().collect();
    entries.sort_by(|a, b| b.1.prompted.cmp(&a.1.prompted));

    let total_calls: usize = entries.iter().map(|(_, s)| s.calls).sum();
    let total_prompted: usize = entries.iter().map(|(_, s)| s.prompted).sum();
    let total_denied: usize = entries.iter().map(|(_, s)| s.denied).sum();
    let total_wait: f64 = entries.iter().map(|(_, s)| s.wait_secs).sum();

    if json {
        let mut json_entries: Vec<_> = entries
            .iter()
            .map(|(detail, s)| {
                serde_json::json!({
                    "detail": detail,
                    "calls": s.calls,
                    "prompted": s.prompted,
                    "denied": s.denied,
                    "wait_secs": round2(s.wait_secs),
                })
            })
            .collect();
        json_entries.push(serde_json::json!({
            "detail": "Total",
            "calls": total_calls,
            "prompted": total_prompted,
            "denied": total_denied,
            "wait_secs": round2(total_wait),
        }));
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Detail"),
            output::Column::right("Calls"),
            output::Column::right("Prompted"),
            output::Column::right("Denied"),
            output::Column::right("Wait"),
        ]);
        for (detail, s) in &entries {
            table.add_row(vec![
                detail.clone(),
                s.calls.to_string(),
                s.prompted.to_string(),
                s.denied.to_string(),
                format_wait(s.wait_secs),
            ]);
        }
        table.print_with_total(&format!(
            "Total: {} calls, {} prompted, {} denied, {} wait",
            total_calls, total_prompted, total_denied, format_wait(total_wait),
        ));
    }

    Ok(())
}

fn run_list(
    events: &[clauson::model::tool_event::ToolEvent],
    tool_filter: Option<&str>,
    event_filter: Option<&str>,
    json: bool,
) -> Result<()> {
    let filtered: Vec<_> = events
        .iter()
        .filter(|e| {
            if let Some(tool) = tool_filter
                && e.tool_name != tool
            {
                return false;
            }
            if let Some(evt) = event_filter
                && e.event.to_string() != evt
            {
                return false;
            }
            true
        })
        .collect();

    if json {
        let json_entries: Vec<_> = filtered
            .iter()
            .map(|e| {
                serde_json::json!({
                    "timestamp": e.timestamp.format("%H:%M:%S%.3f").to_string(),
                    "event": e.event.to_string(),
                    "tool_name": e.tool_name,
                    "tool_use_id": e.tool_use_id.as_deref().unwrap_or(""),
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Time"),
            output::Column::left("Event"),
            output::Column::left("Tool"),
            output::Column::left("Tool Use ID"),
        ]);
        for e in &filtered {
            table.add_row(vec![
                e.timestamp.format("%H:%M:%S").to_string(),
                e.event.to_string(),
                e.tool_name.clone(),
                output::truncate(e.tool_use_id.as_deref().unwrap_or(""), 20),
            ]);
        }
        table.print();
    }

    Ok(())
}

fn run_timeline(
    lifecycles: &[ToolCallLifecycle],
    tool_filter: Option<&str>,
    json: bool,
) -> Result<()> {
    let filtered: Vec<_> = lifecycles
        .iter()
        .filter(|lc| {
            if let Some(tool) = tool_filter {
                lc.tool_name == tool
            } else {
                true
            }
        })
        .collect();

    if json {
        let json_entries: Vec<_> = filtered
            .iter()
            .map(|lc| {
                let detail = extract_tool_detail(&lc.tool_name, &lc.tool_input);
                serde_json::json!({
                    "tool_use_id": lc.tool_use_id.as_deref().unwrap_or(""),
                    "tool_name": lc.tool_name,
                    "detail": detail,
                    "status": lc.status_label(),
                    "wait_secs": lc.permission_wait_secs(),
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Tool Use ID"),
            output::Column::left("Tool"),
            output::Column::left("Detail"),
            output::Column::left("Status"),
            output::Column::right("Wait"),
        ]);
        for lc in &filtered {
            let detail = extract_tool_detail(&lc.tool_name, &lc.tool_input);
            let wait = lc
                .permission_wait_secs()
                .map(|s| format!("{s:.1}s"))
                .unwrap_or_default();
            table.add_row(vec![
                output::truncate(lc.tool_use_id.as_deref().unwrap_or(""), 20),
                lc.tool_name.clone(),
                detail,
                lc.status_label().to_string(),
                wait,
            ]);
        }
        table.print();
    }

    Ok(())
}

fn extract_tool_detail(tool_name: &str, input: &serde_json::Value) -> String {
    let raw = match tool_name {
        "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("(no command)"),
        "Read" | "Write" | "Edit" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("(no path)"),
        "Grep" | "Glob" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("(no pattern)"),
        _ => {
            let s = input.to_string();
            return output::truncate(&s, 40);
        }
    };
    output::truncate(raw, 40)
}

fn pct(part: usize, total: usize) -> f64 {
    if total > 0 {
        (part as f64 / total as f64) * 100.0
    } else {
        0.0
    }
}

fn pct_f64(part: f64, total: f64) -> f64 {
    if total > 0.0 {
        (part / total) * 100.0
    } else {
        0.0
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn format_wait(secs: f64) -> String {
    if secs < 0.05 {
        "0s".to_string()
    } else if secs < 60.0 {
        format!("{secs:.1}s")
    } else {
        let mins = (secs / 60.0).floor() as u64;
        let remainder = secs - (mins as f64 * 60.0);
        format!("{mins}m {remainder:.0}s")
    }
}
