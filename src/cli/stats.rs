use anyhow::Result;
use clap::Subcommand;
use std::collections::HashMap;

use clauson::model::block::{Block, BlockInfo};
use clauson::model::session::Session;
use clauson::model::turn::Turn;
use clauson::model::types::{BlockType, TokenUsage};

use super::output;

#[derive(Subcommand)]
pub enum StatsAction {
    /// Aggregated totals per group (default)
    Summary {
        /// Metric: tokens (default), time, tool-calls
        #[arg(long, default_value = "tokens")]
        metric: String,
        /// Group by: tool (default), type, turn, none
        #[arg(long, default_value = "tool")]
        group_by: String,
        /// Filter to a specific tool (enables drill-down by command/file)
        #[arg(long)]
        tool: Option<String>,
        /// Token component(s), comma-separated: total, input, output, cache-read, cache-create, all
        #[arg(long, default_value = "total")]
        token_type: String,
        /// Sort by: tokens, time, tool-calls (descending). Only applies to --group-by turn.
        #[arg(long)]
        sort_by: Option<String>,
    },
    /// Statistical distribution (min/max/mean/median/p90/p99)
    Distribution {
        /// Metric: tokens (default), time, tool-calls
        #[arg(long, default_value = "tokens")]
        metric: String,
        /// Group by: tool (default), turn
        #[arg(long, default_value = "tool")]
        group_by: String,
        /// Filter to a specific tool (enables drill-down by command/file)
        #[arg(long)]
        tool: Option<String>,
        /// Token component(s), comma-separated: total, input, output, cache-read, cache-create, all
        #[arg(long, default_value = "total")]
        token_type: String,
    },
    /// Sample real turns/blocks at percentile boundaries
    Sample {
        /// Metric: tokens (default), time, tool-calls
        #[arg(long, default_value = "tokens")]
        metric: String,
        /// Comma-separated percentile values
        #[arg(long, default_value = "10,50,90,99")]
        percentiles: String,
        /// Number of items to show near each percentile
        #[arg(long, default_value = "1")]
        count: usize,
        /// Filter to a specific tool (switches to block-level sampling)
        #[arg(long)]
        tool: Option<String>,
        /// Token component(s), comma-separated: total, input, output, cache-read, cache-create, all
        #[arg(long, default_value = "total")]
        token_type: String,
    },
}

pub fn run(session: &Session, action: Option<&StatsAction>, json: bool) -> Result<()> {
    match action {
        None => {
            let types = parse_token_types("all");
            run_summary(session, "tokens", "tool", None, &types, None, json)
        }
        Some(StatsAction::Summary {
            metric,
            group_by,
            tool,
            token_type,
            sort_by,
        }) => {
            let types = parse_token_types(token_type);
            run_summary(
                session,
                metric,
                group_by,
                tool.as_deref(),
                &types,
                sort_by.as_deref(),
                json,
            )
        }
        Some(StatsAction::Distribution {
            metric,
            group_by,
            tool,
            token_type,
        }) => {
            let types = parse_token_types(token_type);
            run_distribution(session, metric, group_by, tool.as_deref(), &types, json)
        }
        Some(StatsAction::Sample {
            metric,
            percentiles,
            count,
            tool,
            token_type,
        }) => {
            let types = parse_token_types(token_type);
            run_sample(
                session,
                metric,
                percentiles,
                *count,
                tool.as_deref(),
                &types,
                json,
            )
        }
    }
}

/// Parse comma-separated token types. `all` expands to all five components.
fn parse_token_types(s: &str) -> Vec<String> {
    if s == "all" {
        return vec![
            "input".into(),
            "output".into(),
            "cache-create".into(),
            "cache-read".into(),
            "total".into(),
        ];
    }
    s.split(',').map(|t| t.trim().to_string()).collect()
}

// ─── Tool detail extraction ─────────────────────────────────────────────────

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
            // Fallback: truncated JSON of full input
            let s = input.to_string();
            return truncate_detail(&s);
        }
    };
    truncate_detail(raw)
}

fn truncate_detail(s: &str) -> String {
    // Take first line only, then truncate
    let first_line = s.lines().next().unwrap_or(s);
    output::truncate(first_line, 60)
}

// ─── Token type selection ────────────────────────────────────────────────────

/// Extract the selected token component from a TokenUsage.
fn token_value(usage: &TokenUsage, token_type: &str) -> u64 {
    match token_type {
        "input" => usage.input_tokens,
        "output" => usage.output_tokens,
        "cache-read" => usage.cache_read_input_tokens,
        "cache-create" => usage.cache_creation_input_tokens,
        _ => usage.total(), // "total"
    }
}

fn token_type_label(token_type: &str) -> &str {
    match token_type {
        "input" => "Input",
        "output" => "Output",
        "cache-read" => "Cache Read",
        "cache-create" => "Cache Create",
        _ => "Total",
    }
}

/// Column header for the percentage column, qualified by type when ambiguous.
fn pct_column_label(primary: &str, num_types: usize) -> String {
    if num_types == 1 && primary == "total" {
        "% of Total".to_string()
    } else {
        format!("% of {}", token_type_label(primary))
    }
}

fn token_type_json_key(token_type: &str) -> &str {
    match token_type {
        "input" => "input_tokens",
        "output" => "output_tokens",
        "cache-read" => "cache_read_input_tokens",
        "cache-create" => "cache_creation_input_tokens",
        _ => "total",
    }
}

// ─── Summary ────────────────────────────────────────────────────────────────

fn run_summary(
    session: &Session,
    metric: &str,
    group_by: &str,
    tool_filter: Option<&str>,
    token_types: &[String],
    sort_by: Option<&str>,
    json: bool,
) -> Result<()> {
    if group_by == "none" {
        return run_summary_aggregate(session, json);
    }
    if let Some(tool_name) = tool_filter {
        return run_summary_tool_detail(session, metric, tool_name, &token_types[0], json);
    }
    match (metric, group_by) {
        ("tokens", "tool") => run_summary_tokens_by_tool(session, token_types, json),
        ("tokens", "type") => run_summary_tokens_by_type(session, token_types, json),
        ("tokens", "turn") => run_summary_tokens_by_turn(session, token_types, sort_by, json),
        ("time", "tool") => run_summary_time_by_tool(session, json),
        ("time", "type") => run_summary_time_by_type(session, json),
        ("time", "turn") => run_summary_time_by_turn(session, sort_by, json),
        ("tool-calls", "tool") => run_summary_tool_calls_by_tool(session, json),
        ("tool-calls", "turn") => run_summary_tool_calls_by_turn(session, sort_by, json),
        _ => {
            eprintln!("Unsupported combination: --metric {metric} --group-by {group_by}");
            eprintln!("Supported group-by values: tool, type, turn, none");
            std::process::exit(1);
        }
    }
}

// summary --group-by none: single aggregate row (replaces old `tokens summary`)
fn run_summary_aggregate(session: &Session, json: bool) -> Result<()> {
    let mut total = TokenUsage::default();
    for &id in session.blocks_of_type(BlockType::Assistant) {
        if let Some(tokens) = session.block(id).tokens() {
            total.merge(tokens);
        }
    }

    if json {
        output::print_json_value(&serde_json::json!({
            "input_tokens": total.input_tokens,
            "output_tokens": total.output_tokens,
            "cache_creation_input_tokens": total.cache_creation_input_tokens,
            "cache_read_input_tokens": total.cache_read_input_tokens,
            "total_input": total.total_input(),
            "total": total.total(),
        }))?;
    } else {
        println!("Token Summary");
        println!("{}", "\u{2500}".repeat(40));
        println!(
            "  Input tokens:            {:>12}",
            output::format_number(total.input_tokens)
        );
        println!(
            "  Cache creation tokens:   {:>12}",
            output::format_number(total.cache_creation_input_tokens)
        );
        println!(
            "  Cache read tokens:       {:>12}",
            output::format_number(total.cache_read_input_tokens)
        );
        println!(
            "  Output tokens:           {:>12}",
            output::format_number(total.output_tokens)
        );
        println!("{}", "\u{2500}".repeat(40));
        println!(
            "  Total:                   {:>12}",
            output::format_number(total.total())
        );
    }

    Ok(())
}

// summary (default): tokens by tool
fn run_summary_tokens_by_tool(
    session: &Session,
    token_types: &[String],
    json: bool,
) -> Result<()> {
    let mut buckets: HashMap<String, TokenUsage> = HashMap::new();

    for &id in session.blocks_of_type(BlockType::Assistant) {
        if let Block::Assistant(a) = session.block(id) {
            if a.tool_calls.is_empty() {
                let entry = buckets.entry("(no tool)".to_string()).or_default();
                entry.merge(&a.tokens);
            } else {
                let n = a.tool_calls.len() as u64;
                let share = TokenUsage {
                    input_tokens: a.tokens.input_tokens / n,
                    output_tokens: a.tokens.output_tokens / n,
                    cache_creation_input_tokens: a.tokens.cache_creation_input_tokens / n,
                    cache_read_input_tokens: a.tokens.cache_read_input_tokens / n,
                };
                for tc in &a.tool_calls {
                    let entry = buckets.entry(tc.tool_name.clone()).or_default();
                    entry.merge(&share);
                }
            }
        }
    }

    // Sort by first token type
    let primary = &token_types[0];
    let grand_total: u64 = buckets.values().map(|t| token_value(t, primary)).sum();
    let mut entries: Vec<_> = buckets.into_iter().collect();
    entries.sort_by(|a, b| token_value(&b.1, primary).cmp(&token_value(&a.1, primary)));

    if json {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|(name, t)| {
                let mut obj = serde_json::json!({ "tool_name": name });
                for tt in token_types {
                    obj[token_type_json_key(tt)] = token_value(t, tt).into();
                }
                let pct = pct_of(token_value(t, primary) as f64, grand_total as f64);
                obj["percent"] = format!("{pct:.1}").into();
                obj
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut columns = vec![output::Column::left("Tool Name")];
        for tt in token_types {
            columns.push(output::Column::right(token_type_label(tt)));
        }
        let pct_label = pct_column_label(primary, token_types.len());
        columns.push(output::Column::right(&pct_label));

        let mut table = output::Table::new(columns);
        for (name, t) in &entries {
            let mut row = vec![name.clone()];
            for tt in token_types {
                row.push(output::format_number(token_value(t, tt)));
            }
            let pct = pct_of(token_value(t, primary) as f64, grand_total as f64);
            row.push(format!("{pct:.1}%"));
            table.add_row(row);
        }
        table.print();
    }

    Ok(())
}

// summary --metric tokens --group-by turn
fn run_summary_tokens_by_turn(
    session: &Session,
    token_types: &[String],
    sort_by: Option<&str>,
    json: bool,
) -> Result<()> {
    let mut turns = session.turns();
    if let Some(sort) = sort_by {
        sort_turns(&mut turns, session, sort);
    }

    if json {
        let json_turns: Vec<_> = turns
            .iter()
            .map(|turn| {
                let user_prompt = if let Block::User(u) = session.block(turn.user_block) {
                    u.content.as_deref().unwrap_or("(no content)")
                } else {
                    "(unknown)"
                };
                let mut obj = serde_json::json!({
                    "turn": turn.index + 1,
                    "user_prompt": output::truncate(user_prompt, 100),
                    "duration_ms": turn.duration_ms,
                });
                for tt in token_types {
                    obj[token_type_json_key(tt)] = token_value(&turn.total_tokens, tt).into();
                }
                obj
            })
            .collect();
        output::print_json(&json_turns)?;
    } else {
        let mut columns = vec![output::Column::right("Turn")];
        for tt in token_types {
            columns.push(output::Column::right(token_type_label(tt)));
        }
        columns.push(output::Column::left("User Prompt"));

        let mut table = output::Table::new(columns);
        for turn in &turns {
            let user_prompt = if let Block::User(u) = session.block(turn.user_block) {
                u.content.as_deref().unwrap_or("(no content)")
            } else {
                "(unknown)"
            };
            let mut row = vec![(turn.index + 1).to_string()];
            for tt in token_types {
                row.push(output::format_number(token_value(&turn.total_tokens, tt)));
            }
            row.push(output::truncate(user_prompt, 40));
            table.add_row(row);
        }
        table.print();
    }

    Ok(())
}

// summary --metric tokens --group-by type
fn run_summary_tokens_by_type(
    session: &Session,
    token_types: &[String],
    json: bool,
) -> Result<()> {
    let mut buckets: HashMap<String, TokenUsage> = HashMap::new();

    for &id in &session.chronological {
        let block = session.block(id);
        if let Some(tokens) = block.tokens() {
            let entry = buckets
                .entry(block.block_type().to_string())
                .or_default();
            entry.merge(tokens);
        }
    }

    let primary = &token_types[0];
    let grand_total: u64 = buckets.values().map(|t| token_value(t, primary)).sum();
    let mut entries: Vec<_> = buckets.into_iter().collect();
    entries.sort_by(|a, b| token_value(&b.1, primary).cmp(&token_value(&a.1, primary)));

    if json {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|(name, t)| {
                let mut obj = serde_json::json!({ "type": name });
                for tt in token_types {
                    obj[token_type_json_key(tt)] = token_value(t, tt).into();
                }
                let pct = pct_of(token_value(t, primary) as f64, grand_total as f64);
                obj["percent"] = format!("{pct:.1}").into();
                obj
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut columns = vec![output::Column::left("Type")];
        for tt in token_types {
            columns.push(output::Column::right(token_type_label(tt)));
        }
        let pct_label = pct_column_label(primary, token_types.len());
        columns.push(output::Column::right(&pct_label));

        let mut table = output::Table::new(columns);
        for (name, t) in &entries {
            let mut row = vec![name.clone()];
            for tt in token_types {
                row.push(output::format_number(token_value(t, tt)));
            }
            let pct = pct_of(token_value(t, primary) as f64, grand_total as f64);
            row.push(format!("{pct:.1}%"));
            table.add_row(row);
        }
        table.print();
    }

    Ok(())
}

// summary --metric time --group-by tool
fn run_summary_time_by_tool(session: &Session, json: bool) -> Result<()> {
    let durations = estimate_durations(session);
    let mut buckets: HashMap<String, TimeAgg> = HashMap::new();

    for bd in &durations {
        let name = match session.block(bd.index) {
            Block::Tool(t) => t.tool_name.clone(),
            _ => continue,
        };
        let entry = buckets.entry(name).or_default();
        entry.count += 1;
        entry.total_ms += bd.duration_ms;
    }

    print_time_agg("Tool Name", &buckets, json)
}

// summary --metric time --group-by type
fn run_summary_time_by_type(session: &Session, json: bool) -> Result<()> {
    let durations = estimate_durations(session);
    let mut buckets: HashMap<String, TimeAgg> = HashMap::new();

    for bd in &durations {
        let name = session.block(bd.index).block_type().to_string();
        let entry = buckets.entry(name).or_default();
        entry.count += 1;
        entry.total_ms += bd.duration_ms;
    }

    print_time_agg("Type", &buckets, json)
}

// summary --metric time --group-by turn
fn run_summary_time_by_turn(session: &Session, sort_by: Option<&str>, json: bool) -> Result<()> {
    let mut turns = session.turns();
    if let Some(sort) = sort_by {
        sort_turns(&mut turns, session, sort);
    }
    let durations = estimate_durations(session);
    let dur_map: HashMap<usize, f64> = durations
        .iter()
        .map(|bd| (bd.index, bd.duration_ms))
        .collect();

    if json {
        let json_turns: Vec<_> = turns
            .iter()
            .map(|turn| {
                let tool_time: f64 = turn
                    .tool_blocks
                    .iter()
                    .map(|&id| dur_map.get(&id).copied().unwrap_or(0.0))
                    .sum();
                let turn_duration = turn.duration_ms.map(|d| d as f64);
                serde_json::json!({
                    "turn": turn.index + 1,
                    "duration_ms": turn_duration,
                    "tool_time_ms": tool_time,
                    "tool_count": turn.tool_blocks.len(),
                })
            })
            .collect();
        output::print_json(&json_turns)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::right("Turn"),
            output::Column::right("Duration"),
            output::Column::right("Tool Time"),
            output::Column::right("Tools"),
        ]);
        for turn in &turns {
            let tool_time: f64 = turn
                .tool_blocks
                .iter()
                .map(|&id| dur_map.get(&id).copied().unwrap_or(0.0))
                .sum();
            let duration = turn
                .duration_ms
                .map(|d| format_duration_ms(d as f64))
                .unwrap_or_default();
            table.add_row(vec![
                (turn.index + 1).to_string(),
                duration,
                format_duration_ms(tool_time),
                turn.tool_blocks.len().to_string(),
            ]);
        }
        table.print();
    }

    Ok(())
}

// summary --metric tool-calls --group-by tool
fn run_summary_tool_calls_by_tool(session: &Session, json: bool) -> Result<()> {
    let mut buckets: HashMap<String, usize> = HashMap::new();

    for &id in session.blocks_of_type(BlockType::Tool) {
        if let Block::Tool(t) = session.block(id) {
            *buckets.entry(t.tool_name.clone()).or_default() += 1;
        }
    }

    let grand_total: usize = buckets.values().sum();
    let mut entries: Vec<_> = buckets.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    if json {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|(name, count)| {
                let pct = pct_of(*count as f64, grand_total as f64);
                serde_json::json!({
                    "tool_name": name,
                    "count": count,
                    "percent": format!("{pct:.1}"),
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Tool Name"),
            output::Column::right("Count"),
            output::Column::right("% of Total"),
        ]);
        for (name, count) in &entries {
            let pct = pct_of(*count as f64, grand_total as f64);
            table.add_row(vec![
                name.clone(),
                count.to_string(),
                format!("{pct:.1}%"),
            ]);
        }
        table.print();
    }

    Ok(())
}

// summary --metric tool-calls --group-by turn
fn run_summary_tool_calls_by_turn(
    session: &Session,
    sort_by: Option<&str>,
    json: bool,
) -> Result<()> {
    let mut turns = session.turns();
    if let Some(sort) = sort_by {
        sort_turns(&mut turns, session, sort);
    }

    if json {
        let json_turns: Vec<_> = turns
            .iter()
            .map(|turn| {
                let user_prompt = if let Block::User(u) = session.block(turn.user_block) {
                    u.content.as_deref().unwrap_or("(no content)")
                } else {
                    "(unknown)"
                };
                serde_json::json!({
                    "turn": turn.index + 1,
                    "tool_calls": turn.tool_blocks.len(),
                    "user_prompt": output::truncate(user_prompt, 100),
                })
            })
            .collect();
        output::print_json(&json_turns)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::right("Turn"),
            output::Column::right("Tool Calls"),
            output::Column::left("User Prompt"),
        ]);
        for turn in &turns {
            let user_prompt = if let Block::User(u) = session.block(turn.user_block) {
                u.content.as_deref().unwrap_or("(no content)")
            } else {
                "(unknown)"
            };
            table.add_row(vec![
                (turn.index + 1).to_string(),
                turn.tool_blocks.len().to_string(),
                output::truncate(user_prompt, 50),
            ]);
        }
        table.print();
    }

    Ok(())
}

// summary --tool NAME: drill-down into a specific tool's invocations
fn run_summary_tool_detail(
    session: &Session,
    metric: &str,
    tool_name: &str,
    token_type: &str,
    json: bool,
) -> Result<()> {
    let durations = estimate_durations(session);
    let dur_map: HashMap<usize, f64> = durations
        .iter()
        .map(|bd| (bd.index, bd.duration_ms))
        .collect();

    // Collect (detail, metric_value) for each invocation of this tool
    struct DetailAgg {
        count: usize,
        total: f64,
    }
    let mut buckets: HashMap<String, DetailAgg> = HashMap::new();

    for &id in session.blocks_of_type(BlockType::Tool) {
        if let Block::Tool(t) = session.block(id) {
            if t.tool_name != tool_name {
                continue;
            }
            let detail = extract_tool_detail(&t.tool_name, &t.input);
            let value = match metric {
                "time" => dur_map.get(&id).copied().unwrap_or(0.0),
                "tool-calls" => 1.0,
                _ => find_tool_tokens(session, &t.tool_use_id, token_type),
            };
            let entry = buckets.entry(detail).or_insert(DetailAgg {
                count: 0,
                total: 0.0,
            });
            entry.count += 1;
            entry.total += value;
        }
    }

    let grand_total: f64 = buckets.values().map(|a| a.total).sum();
    let mut entries: Vec<_> = buckets.into_iter().collect();
    entries.sort_by(|a, b| b.1.total.partial_cmp(&a.1.total).unwrap());

    let format_val = metric_formatter(metric);

    if json {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|(detail, agg)| {
                let avg = if agg.count > 0 {
                    agg.total / agg.count as f64
                } else {
                    0.0
                };
                let pct = pct_of(agg.total, grand_total);
                serde_json::json!({
                    "detail": detail,
                    "count": agg.count,
                    "total": agg.total,
                    "avg": avg,
                    "percent": format!("{pct:.1}"),
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Detail"),
            output::Column::right("Count"),
            output::Column::right("Total"),
            output::Column::right("Avg"),
            output::Column::right("%"),
        ]);
        for (detail, agg) in &entries {
            let avg = if agg.count > 0 {
                agg.total / agg.count as f64
            } else {
                0.0
            };
            let pct = pct_of(agg.total, grand_total);
            table.add_row(vec![
                detail.clone(),
                agg.count.to_string(),
                format_val(agg.total),
                format_val(avg),
                format!("{pct:.1}%"),
            ]);
        }
        table.print();
    }

    Ok(())
}

// ─── Distribution ───────────────────────────────────────────────────────────

fn run_distribution(
    session: &Session,
    metric: &str,
    group_by: &str,
    tool_filter: Option<&str>,
    token_types: &[String],
    json: bool,
) -> Result<()> {
    if metric != "tokens" {
        if let Some(tool_name) = tool_filter {
            return run_dist_tool_detail_nontoken(session, metric, tool_name, json);
        }
        return match (metric, group_by) {
            ("time", "tool") => run_dist_time_by_tool(session, json),
            ("time", "turn") => run_dist_time_by_turn(session, json),
            ("tool-calls", "turn") => run_dist_tool_calls_by_turn(session, json),
            _ => {
                eprintln!("Unsupported combination: --metric {metric} --group-by {group_by}");
                eprintln!(
                    "Supported: tokens/tool, time/tool, tokens/turn, time/turn, tool-calls/turn"
                );
                std::process::exit(1);
            }
        };
    }

    if let Some(tool_name) = tool_filter {
        return run_dist_tool_detail_tokens(session, tool_name, token_types, json);
    }
    match group_by {
        "tool" => run_dist_tokens_by_tool(session, token_types, json),
        "turn" => run_dist_tokens_by_turn(session, token_types, json),
        _ => {
            eprintln!("Unsupported --group-by {group_by} for tokens distribution");
            eprintln!("Supported: tool, turn");
            std::process::exit(1);
        }
    }
}

fn run_dist_tokens_by_tool(session: &Session, token_types: &[String], json: bool) -> Result<()> {
    // tool -> [type_0_values, type_1_values, ...]
    let mut tool_values: HashMap<String, Vec<Vec<f64>>> = HashMap::new();

    for &id in session.blocks_of_type(BlockType::Assistant) {
        if let Block::Assistant(a) = session.block(id) {
            if a.tool_calls.is_empty() {
                continue;
            }
            let n = a.tool_calls.len() as f64;
            for tc in &a.tool_calls {
                let type_vals = tool_values
                    .entry(tc.tool_name.clone())
                    .or_insert_with(|| token_types.iter().map(|_| Vec::new()).collect());
                for (i, tt) in token_types.iter().enumerate() {
                    type_vals[i].push(token_value(&a.tokens, tt) as f64 / n);
                }
            }
        }
    }

    print_dist_grouped_tokens("Tool Name", &mut tool_values, token_types, json)
}

fn run_dist_time_by_tool(session: &Session, json: bool) -> Result<()> {
    let durations = estimate_durations(session);
    let mut tool_values: HashMap<String, Vec<f64>> = HashMap::new();

    for bd in &durations {
        if let Block::Tool(t) = session.block(bd.index) {
            tool_values
                .entry(t.tool_name.clone())
                .or_default()
                .push(bd.duration_ms);
        }
    }

    print_dist_grouped("Tool Name", &mut tool_values, json, format_duration_ms)
}

fn run_dist_tokens_by_turn(session: &Session, token_types: &[String], json: bool) -> Result<()> {
    let turns = session.turns();
    let mut type_values: Vec<Vec<f64>> = token_types.iter().map(|_| Vec::new()).collect();
    for turn in &turns {
        for (i, tt) in token_types.iter().enumerate() {
            type_values[i].push(token_value(&turn.total_tokens, tt) as f64);
        }
    }
    print_single_dist_tokens("tokens per turn", &mut type_values, token_types, json)
}

fn run_dist_time_by_turn(session: &Session, json: bool) -> Result<()> {
    let turns = session.turns();
    let mut values: Vec<f64> = turns
        .iter()
        .map(|t| t.duration_ms.unwrap_or(0) as f64)
        .collect();

    print_single_dist_output("time per turn", &mut values, json, format_duration_ms)
}

fn run_dist_tool_calls_by_turn(session: &Session, json: bool) -> Result<()> {
    let turns = session.turns();
    let mut values: Vec<f64> = turns.iter().map(|t| t.tool_blocks.len() as f64).collect();

    print_single_dist_output("tool calls per turn", &mut values, json, |v| {
        format!("{v:.0}")
    })
}

// distribution --tool NAME (non-token metrics): per-invocation stats with detail extraction
fn run_dist_tool_detail_nontoken(
    session: &Session,
    metric: &str,
    tool_name: &str,
    json: bool,
) -> Result<()> {
    let durations = estimate_durations(session);
    let dur_map: HashMap<usize, f64> = durations
        .iter()
        .map(|bd| (bd.index, bd.duration_ms))
        .collect();

    let mut detail_values: HashMap<String, Vec<f64>> = HashMap::new();

    for &id in session.blocks_of_type(BlockType::Tool) {
        if let Block::Tool(t) = session.block(id) {
            if t.tool_name != tool_name {
                continue;
            }
            let detail = extract_tool_detail(&t.tool_name, &t.input);
            let value = match metric {
                "time" => dur_map.get(&id).copied().unwrap_or(0.0),
                _ => 1.0,
            };
            detail_values.entry(detail).or_default().push(value);
        }
    }

    let format_val = metric_formatter(metric);
    print_dist_grouped("Detail", &mut detail_values, json, format_val)
}

// distribution --tool NAME (token metric): multi-type per-invocation stats
fn run_dist_tool_detail_tokens(
    session: &Session,
    tool_name: &str,
    token_types: &[String],
    json: bool,
) -> Result<()> {
    let mut detail_values: HashMap<String, Vec<Vec<f64>>> = HashMap::new();

    for &id in session.blocks_of_type(BlockType::Tool) {
        if let Block::Tool(t) = session.block(id) {
            if t.tool_name != tool_name {
                continue;
            }
            let detail = extract_tool_detail(&t.tool_name, &t.input);
            let type_vals = detail_values
                .entry(detail)
                .or_insert_with(|| token_types.iter().map(|_| Vec::new()).collect());
            for (i, tt) in token_types.iter().enumerate() {
                type_vals[i].push(find_tool_tokens(session, &t.tool_use_id, tt));
            }
        }
    }

    print_dist_grouped_tokens("Detail", &mut detail_values, token_types, json)
}

// ─── Sample ─────────────────────────────────────────────────────────────────

fn run_sample(
    session: &Session,
    metric: &str,
    percentiles_str: &str,
    count: usize,
    tool_filter: Option<&str>,
    token_types: &[String],
    json: bool,
) -> Result<()> {
    let percentiles: Vec<f64> = percentiles_str
        .split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect();

    // For non-token metrics, use a single "total" type (value doesn't depend on type)
    let effective_types: Vec<String> = if metric != "tokens" {
        vec!["total".to_string()]
    } else {
        token_types.to_vec()
    };

    if let Some(tool_name) = tool_filter {
        return run_sample_blocks(
            session,
            metric,
            &percentiles,
            count,
            tool_name,
            &effective_types,
            json,
        );
    }
    run_sample_turns(session, metric, &percentiles, count, &effective_types, json)
}

// sample (default): sample turns
fn run_sample_turns(
    session: &Session,
    metric: &str,
    percentiles: &[f64],
    count: usize,
    token_types: &[String],
    json: bool,
) -> Result<()> {
    let turns = session.turns();
    if turns.is_empty() {
        println!("No turns found.");
        return Ok(());
    }

    // Build scored items: (turn_number, primary_value, per_type_values, prompt)
    let scored: Vec<(usize, f64, Vec<f64>, String)> = turns
        .iter()
        .map(|turn| {
            let values: Vec<f64> = token_types
                .iter()
                .map(|tt| match metric {
                    "time" => turn.duration_ms.unwrap_or(0) as f64,
                    "tool-calls" => turn.tool_blocks.len() as f64,
                    _ => token_value(&turn.total_tokens, tt) as f64,
                })
                .collect();
            let primary = values[0];
            let prompt = if let Block::User(u) = session.block(turn.user_block) {
                u.content
                    .as_deref()
                    .unwrap_or("(no content)")
                    .to_string()
            } else {
                "(unknown)".to_string()
            };
            (turn.index + 1, primary, values, prompt)
        })
        .collect();

    let primary_values: Vec<f64> = scored.iter().map(|s| s.1).collect();
    let mut sorted_values = primary_values.clone();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let format_val = metric_formatter(metric);
    if json {
        let mut json_entries = Vec::new();
        for &p in percentiles {
            let threshold = percentile_value(&sorted_values, p);
            let indices = find_closest_indices(&primary_values, threshold, count);
            for idx in indices {
                let (turn_num, _, ref values, ref prompt) = scored[idx];
                let mut obj = serde_json::json!({
                    "percentile": format!("p{}", p as u64),
                    "turn": turn_num,
                    "user_prompt": output::truncate(prompt, 60),
                });
                for (i, tt) in token_types.iter().enumerate() {
                    obj[token_type_json_key(tt)] = values[i].into();
                }
                json_entries.push(obj);
            }
        }
        output::print_json(&json_entries)?;
    } else {
        let mut columns = vec![
            output::Column::left("Percentile"),
            output::Column::right("Turn"),
        ];
        for tt in token_types {
            columns.push(output::Column::right(token_type_label(tt)));
        }
        columns.push(output::Column::left("User Prompt"));

        let mut table = output::Table::new(columns);
        for &p in percentiles {
            let threshold = percentile_value(&sorted_values, p);
            let indices = find_closest_indices(&primary_values, threshold, count);
            for idx in indices {
                let (turn_num, _, ref values, ref prompt) = scored[idx];
                let mut row = vec![format!("p{}", p as u64), turn_num.to_string()];
                for v in values {
                    row.push(format_val(*v));
                }
                row.push(output::truncate(prompt, 50));
                table.add_row(row);
            }
        }
        table.print();
    }

    Ok(())
}

// sample --tool NAME: sample individual tool blocks
fn run_sample_blocks(
    session: &Session,
    metric: &str,
    percentiles: &[f64],
    count: usize,
    tool_name: &str,
    token_types: &[String],
    json: bool,
) -> Result<()> {
    let durations = estimate_durations(session);
    let dur_map: HashMap<usize, f64> = durations
        .iter()
        .map(|bd| (bd.index, bd.duration_ms))
        .collect();

    // Build scored items: (block_uuid, primary_value, per_type_values, detail)
    let scored: Vec<(String, f64, Vec<f64>, String)> = session
        .blocks_of_type(BlockType::Tool)
        .iter()
        .filter_map(|&id| {
            if let Block::Tool(t) = session.block(id) {
                if t.tool_name != tool_name {
                    return None;
                }
                let values: Vec<f64> = token_types
                    .iter()
                    .map(|tt| match metric {
                        "time" => dur_map.get(&id).copied().unwrap_or(0.0),
                        "tool-calls" => 1.0,
                        _ => find_tool_tokens(session, &t.tool_use_id, tt),
                    })
                    .collect();
                let primary = values[0];
                let detail = extract_tool_detail(&t.tool_name, &t.input);
                Some((t.tool_use_id.clone(), primary, values, detail))
            } else {
                None
            }
        })
        .collect();

    if scored.is_empty() {
        println!("No blocks found for tool '{tool_name}'.");
        return Ok(());
    }

    let primary_values: Vec<f64> = scored.iter().map(|s| s.1).collect();
    let mut sorted_values = primary_values.clone();
    sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let format_val = metric_formatter(metric);

    if json {
        let mut json_entries = Vec::new();
        for &p in percentiles {
            let threshold = percentile_value(&sorted_values, p);
            let indices = find_closest_indices(&primary_values, threshold, count);
            for idx in indices {
                let (ref uuid, _, ref values, ref detail) = scored[idx];
                let mut obj = serde_json::json!({
                    "percentile": format!("p{}", p as u64),
                    "block_id": uuid,
                    "detail": detail,
                });
                for (i, tt) in token_types.iter().enumerate() {
                    obj[token_type_json_key(tt)] = values[i].into();
                }
                json_entries.push(obj);
            }
        }
        output::print_json(&json_entries)?;
    } else {
        let mut columns = vec![output::Column::left("Percentile")];
        for tt in token_types {
            columns.push(output::Column::right(token_type_label(tt)));
        }
        columns.push(output::Column::left("Block ID"));
        columns.push(output::Column::left("Detail"));

        let mut table = output::Table::new(columns);
        for &p in percentiles {
            let threshold = percentile_value(&sorted_values, p);
            let indices = find_closest_indices(&primary_values, threshold, count);
            for idx in indices {
                let (ref uuid, _, ref values, ref detail) = scored[idx];
                let mut row = vec![format!("p{}", p as u64)];
                for v in values {
                    row.push(format_val(*v));
                }
                row.push(output::truncate(uuid, 20));
                row.push(output::truncate(detail, 40));
                table.add_row(row);
            }
        }
        table.print();
    }

    Ok(())
}

// ─── Shared helpers ─────────────────────────────────────────────────────────

struct BlockDuration {
    index: usize,
    duration_ms: f64,
}

fn estimate_durations(session: &Session) -> Vec<BlockDuration> {
    let chrono = &session.chronological;
    let mut durations = Vec::with_capacity(chrono.len());
    for (i, &id) in chrono.iter().enumerate() {
        let ts = session.block(id).timestamp();
        let dur = if i + 1 < chrono.len() {
            let next_ts = session.block(chrono[i + 1]).timestamp();
            (next_ts - ts).num_milliseconds().max(0) as f64
        } else {
            0.0
        };
        durations.push(BlockDuration {
            index: id,
            duration_ms: dur,
        });
    }
    durations
}

#[derive(Default)]
struct TimeAgg {
    count: usize,
    total_ms: f64,
}

fn print_time_agg(label: &str, buckets: &HashMap<String, TimeAgg>, json: bool) -> Result<()> {
    let grand_total: f64 = buckets.values().map(|a| a.total_ms).sum();
    let mut entries: Vec<_> = buckets.iter().collect();
    entries.sort_by(|a, b| b.1.total_ms.partial_cmp(&a.1.total_ms).unwrap());

    if json {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|(name, agg)| {
                let avg = safe_div(agg.total_ms, agg.count as f64);
                let pct = pct_of(agg.total_ms, grand_total);
                serde_json::json!({
                    "name": name,
                    "count": agg.count,
                    "total_ms": agg.total_ms,
                    "avg_ms": avg,
                    "percent": format!("{pct:.1}"),
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left(label),
            output::Column::right("Count"),
            output::Column::right("Total"),
            output::Column::right("Avg"),
            output::Column::right("% of Total"),
        ]);
        for (name, agg) in &entries {
            let avg = safe_div(agg.total_ms, agg.count as f64);
            let pct = pct_of(agg.total_ms, grand_total);
            table.add_row(vec![
                (*name).clone(),
                agg.count.to_string(),
                format_duration_ms(agg.total_ms),
                format_duration_ms(avg),
                format!("{pct:.1}%"),
            ]);
        }
        table.print();
    }

    Ok(())
}

fn format_duration_ms(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{ms:.0}ms")
    } else {
        format!("{:.1}s", ms / 1000.0)
    }
}

// ─── Distribution helpers ───────────────────────────────────────────────────

struct DistStats {
    count: usize,
    min: f64,
    max: f64,
    mean: f64,
    median: f64,
    p90: f64,
    p99: f64,
    sum: f64,
}

fn percentile_value(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = (p / 100.0) * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;
    sorted[lower] * (1.0 - frac) + sorted[upper] * frac
}

fn compute_stats(values: &mut [f64]) -> Option<DistStats> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let count = values.len();
    let sum: f64 = values.iter().sum();
    let mean = sum / count as f64;
    Some(DistStats {
        count,
        min: values[0],
        max: values[count - 1],
        mean,
        median: percentile_value(values, 50.0),
        p90: percentile_value(values, 90.0),
        p99: percentile_value(values, 99.0),
        sum,
    })
}

fn print_dist_grouped(
    label: &str,
    tool_values: &mut HashMap<String, Vec<f64>>,
    json: bool,
    fmt: impl Fn(f64) -> String,
) -> Result<()> {
    let mut entries: Vec<(String, DistStats)> = tool_values
        .iter_mut()
        .filter_map(|(name, vals)| compute_stats(vals).map(|s| (name.clone(), s)))
        .collect();
    entries.sort_by(|a, b| b.1.sum.partial_cmp(&a.1.sum).unwrap());

    if json {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|(name, s)| {
                serde_json::json!({
                    "name": name,
                    "count": s.count,
                    "min": s.min,
                    "max": s.max,
                    "mean": s.mean,
                    "median": s.median,
                    "p90": s.p90,
                    "p99": s.p99,
                    "sum": s.sum,
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left(label),
            output::Column::right("Count"),
            output::Column::right("Min"),
            output::Column::right("Max"),
            output::Column::right("Mean"),
            output::Column::right("Median"),
            output::Column::right("p90"),
            output::Column::right("p99"),
        ]);
        for (name, s) in &entries {
            table.add_row(vec![
                name.clone(),
                s.count.to_string(),
                fmt(s.min),
                fmt(s.max),
                fmt(s.mean),
                fmt(s.median),
                fmt(s.p90),
                fmt(s.p99),
            ]);
        }
        table.print();
    }

    Ok(())
}

fn print_single_dist_output(
    description: &str,
    values: &mut [f64],
    json: bool,
    fmt: impl Fn(f64) -> String,
) -> Result<()> {
    if let Some(stats) = compute_stats(values) {
        if json {
            output::print_json_value(&serde_json::json!({
                "metric": description,
                "count": stats.count,
                "min": stats.min,
                "max": stats.max,
                "mean": stats.mean,
                "median": stats.median,
                "p90": stats.p90,
                "p99": stats.p99,
                "sum": stats.sum,
            }))?;
        } else {
            println!(
                "Distribution of {description} across {} items:",
                stats.count
            );
            println!();
            let mut table = output::Table::new(vec![
                output::Column::left("Stat"),
                output::Column::right("Value"),
            ]);
            table.add_row(vec!["Min".to_string(), fmt(stats.min)]);
            table.add_row(vec!["Max".to_string(), fmt(stats.max)]);
            table.add_row(vec!["Mean".to_string(), fmt(stats.mean)]);
            table.add_row(vec!["Median".to_string(), fmt(stats.median)]);
            table.add_row(vec!["p90".to_string(), fmt(stats.p90)]);
            table.add_row(vec!["p99".to_string(), fmt(stats.p99)]);
            table.add_row(vec!["Sum".to_string(), fmt(stats.sum)]);
            table.print();
        }
    } else {
        println!("No data found.");
    }
    Ok(())
}

// ─── Multi-type distribution helpers ────────────────────────────────────────

fn dist_stat_values(s: &DistStats) -> [f64; 6] {
    [s.min, s.max, s.mean, s.median, s.p90, s.p99]
}

fn print_dist_grouped_tokens(
    label: &str,
    tool_values: &mut HashMap<String, Vec<Vec<f64>>>,
    token_types: &[String],
    json: bool,
) -> Result<()> {
    let mut entries: Vec<(String, Vec<DistStats>)> = tool_values
        .iter_mut()
        .filter_map(|(name, type_vals)| {
            let stats: Vec<DistStats> = type_vals
                .iter_mut()
                .filter_map(|vals| compute_stats(vals))
                .collect();
            if stats.len() == token_types.len() {
                Some((name.clone(), stats))
            } else {
                None
            }
        })
        .collect();

    // Sort by sum of first type (descending)
    entries.sort_by(|a, b| b.1[0].sum.partial_cmp(&a.1[0].sum).unwrap());

    if json {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|(name, per_type)| {
                let mut obj = serde_json::json!({
                    "name": name,
                    "count": per_type[0].count,
                });
                for (i, tt) in token_types.iter().enumerate() {
                    let s = &per_type[i];
                    obj[token_type_json_key(tt)] = serde_json::json!({
                        "min": s.min, "max": s.max, "mean": s.mean,
                        "median": s.median, "p90": s.p90, "p99": s.p99, "sum": s.sum,
                    });
                }
                obj
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let qualify = token_types.len() > 1 || token_types[0] != "total";
        let stat_names = ["Count", "Min", "Max", "Mean", "Median", "p90", "p99"];
        let mut columns = vec![output::Column::left(label)];
        for &stat in &stat_names {
            if qualify && stat != "Count" {
                for tt in token_types {
                    columns.push(output::Column::right(&format!(
                        "{} ({})",
                        stat,
                        token_type_label(tt)
                    )));
                }
            } else {
                columns.push(output::Column::right(stat));
            }
        }

        let mut table = output::Table::new(columns);
        for (name, per_type) in &entries {
            let mut row = vec![name.clone()];
            // Count (same across all types)
            row.push(per_type[0].count.to_string());
            // Interleaved stat values: for each stat, add values for all types
            let type_stat_vals: Vec<[f64; 6]> =
                per_type.iter().map(dist_stat_values).collect();
            for stat_idx in 0..6 {
                for type_vals in &type_stat_vals {
                    row.push(output::format_number(type_vals[stat_idx] as u64));
                }
            }
            table.add_row(row);
        }
        table.print();
    }

    Ok(())
}

fn print_single_dist_tokens(
    description: &str,
    type_values: &mut [Vec<f64>],
    token_types: &[String],
    json: bool,
) -> Result<()> {
    let all_stats: Vec<Option<DistStats>> = type_values
        .iter_mut()
        .map(|vals| compute_stats(vals))
        .collect();

    let first_stats = match &all_stats[0] {
        Some(s) => s,
        None => {
            println!("No data found.");
            return Ok(());
        }
    };

    if json {
        let mut obj = serde_json::json!({
            "metric": description,
            "count": first_stats.count,
        });
        for (i, tt) in token_types.iter().enumerate() {
            if let Some(s) = &all_stats[i] {
                obj[token_type_json_key(tt)] = serde_json::json!({
                    "min": s.min, "max": s.max, "mean": s.mean,
                    "median": s.median, "p90": s.p90, "p99": s.p99, "sum": s.sum,
                });
            }
        }
        output::print_json_value(&obj)?;
    } else {
        let multi = token_types.len() > 1;
        println!(
            "Distribution of {description} across {} items:",
            first_stats.count
        );
        println!();

        let mut columns = vec![output::Column::left("Stat")];
        if multi {
            for tt in token_types {
                columns.push(output::Column::right(token_type_label(tt)));
            }
        } else {
            columns.push(output::Column::right(token_type_label(&token_types[0])));
        }

        let stat_names = ["Min", "Max", "Mean", "Median", "p90", "p99", "Sum"];
        let mut table = output::Table::new(columns);
        for (stat_idx, &stat_name) in stat_names.iter().enumerate() {
            let mut row = vec![stat_name.to_string()];
            for s in all_stats.iter().flatten() {
                let val = match stat_idx {
                    0 => s.min,
                    1 => s.max,
                    2 => s.mean,
                    3 => s.median,
                    4 => s.p90,
                    5 => s.p99,
                    _ => s.sum,
                };
                row.push(output::format_number(val as u64));
            }
            table.add_row(row);
        }
        table.print();
    }

    Ok(())
}

// ─── Sample helpers ─────────────────────────────────────────────────────────

/// Find the N indices of items whose values are closest to the threshold.
fn find_closest_indices(values: &[f64], threshold: f64, count: usize) -> Vec<usize> {
    let mut with_dist: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| (i, (v - threshold).abs()))
        .collect();
    with_dist.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    with_dist.into_iter().take(count).map(|(i, _)| i).collect()
}

// ─── Utility ────────────────────────────────────────────────────────────────

/// Sort turns in-place by the given metric (descending).
fn sort_turns(turns: &mut [Turn], session: &Session, sort_by: &str) {
    let durations: HashMap<usize, f64> = if sort_by == "time" {
        estimate_durations(session)
            .into_iter()
            .map(|bd| (bd.index, bd.duration_ms))
            .collect()
    } else {
        HashMap::new()
    };
    turns.sort_by(|a, b| {
        let va = turn_sort_value(a, &durations, sort_by);
        let vb = turn_sort_value(b, &durations, sort_by);
        vb.partial_cmp(&va).unwrap()
    });
}

fn turn_sort_value(turn: &Turn, dur_map: &HashMap<usize, f64>, sort_by: &str) -> f64 {
    match sort_by {
        "time" => turn
            .duration_ms
            .map(|d| d as f64)
            .unwrap_or_else(|| {
                turn.all_blocks
                    .iter()
                    .map(|&id| dur_map.get(&id).copied().unwrap_or(0.0))
                    .sum()
            }),
        "tool-calls" => turn.tool_blocks.len() as f64,
        _ => turn.total_tokens.total() as f64, // "tokens" or default
    }
}

fn pct_of(part: f64, total: f64) -> f64 {
    if total > 0.0 {
        (part / total) * 100.0
    } else {
        0.0
    }
}

fn safe_div(a: f64, b: f64) -> f64 {
    if b > 0.0 {
        a / b
    } else {
        0.0
    }
}

/// Look up approximate token usage for a specific tool invocation by its tool_use_id.
fn find_tool_tokens(session: &Session, tool_use_id: &str, token_type: &str) -> f64 {
    for &id in session.blocks_of_type(BlockType::Assistant) {
        if let Block::Assistant(a) = session.block(id) {
            for tc in &a.tool_calls {
                if tc.tool_use_id == tool_use_id {
                    // Split tokens evenly across tool calls in this assistant block
                    return token_value(&a.tokens, token_type) as f64
                        / a.tool_calls.len() as f64;
                }
            }
        }
    }
    0.0
}

/// Return a formatting closure based on the metric name.
fn metric_formatter(metric: &str) -> Box<dyn Fn(f64) -> String> {
    match metric {
        "time" => Box::new(format_duration_ms),
        "tool-calls" => Box::new(|v| format!("{v:.0}")),
        _ => Box::new(|v| output::format_number(v as u64)),
    }
}
