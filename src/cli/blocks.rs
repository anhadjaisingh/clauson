use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;
use std::collections::HashMap;

use clauson::model::block::{Block, BlockInfo};
use clauson::model::session::Session;
use clauson::model::types::BlockType;

use super::output;

#[derive(Subcommand)]
pub enum BlocksAction {
    /// List blocks
    List {
        /// Filter by block type (user, assistant, tool, system)
        #[arg(long, value_name = "TYPE")]
        r#type: Option<String>,

        /// Filter by turn number (1-indexed)
        #[arg(long)]
        turn: Option<usize>,

        /// Filter by tool name
        #[arg(long, value_name = "NAME")]
        tool_name: Option<String>,
    },
    /// Count blocks grouped by type or tool
    Count {
        /// Group by: type, tool
        #[arg(long, value_name = "FIELD", default_value = "type")]
        group_by: String,
    },
    /// Show details of a specific block by UUID
    Show {
        /// Block UUID (prefix match supported)
        uuid: String,
    },
}

pub fn run(session: &Session, action: Option<&BlocksAction>, json: bool, raw: bool) -> Result<()> {
    match action {
        None | Some(BlocksAction::List { .. }) => {
            let (type_filter, turn_filter, tool_name_filter) = match action {
                Some(BlocksAction::List {
                    r#type,
                    turn,
                    tool_name,
                }) => (r#type.as_deref(), *turn, tool_name.as_deref()),
                _ => (None, None, None),
            };
            run_list(session, type_filter, turn_filter, tool_name_filter, json, raw)
        }
        Some(BlocksAction::Count { group_by }) => run_count(session, group_by, json),
        Some(BlocksAction::Show { uuid }) => run_show(session, uuid, json, raw),
    }
}

#[derive(Serialize)]
struct BlockSummary {
    index: usize,
    block_type: String,
    uuid: String,
    timestamp: String,
    summary: String,
}

fn block_summary(id: usize, block: &Block) -> BlockSummary {
    let summary = match block {
        Block::User(u) => {
            let content = u
                .content
                .as_deref()
                .unwrap_or("(no content)");
            output::truncate(content, 60)
        }
        Block::Assistant(a) => {
            let mut parts = vec![];
            if let Some(content) = &a.content {
                parts.push(output::truncate(content, 40));
            }
            if !a.tool_calls.is_empty() {
                parts.push(format!("[{} tool calls]", a.tool_calls.len()));
            }
            if parts.is_empty() {
                "(empty)".to_string()
            } else {
                parts.join(" ")
            }
        }
        Block::Tool(t) => {
            let status = if t.is_error { " ERROR" } else { "" };
            format!("{}{}", t.tool_name, status)
        }
        Block::System(s) => format!("{:?}", s.subtype),
    };

    BlockSummary {
        index: id,
        block_type: block.block_type().to_string(),
        uuid: block.uuid().to_string(),
        timestamp: block.timestamp().format("%H:%M:%S").to_string(),
        summary,
    }
}

fn run_list(
    session: &Session,
    type_filter: Option<&str>,
    turn_filter: Option<usize>,
    tool_name_filter: Option<&str>,
    json: bool,
    _raw: bool,
) -> Result<()> {
    let type_filter = type_filter
        .map(|t| match t {
            "user" => BlockType::User,
            "assistant" => BlockType::Assistant,
            "tool" => BlockType::Tool,
            "system" => BlockType::System,
            _ => BlockType::User,
        });

    let turn_blocks: Option<Vec<usize>> = turn_filter.map(|n| {
        let turns = session.turns();
        if n > 0 && n <= turns.len() {
            turns[n - 1].all_blocks.clone()
        } else {
            vec![]
        }
    });

    let tool_blocks: Option<Vec<usize>> = tool_name_filter.map(|name| {
        session.tools_by_name(name).to_vec()
    });

    let mut summaries = Vec::new();
    for &id in &session.chronological {
        let block = session.block(id);

        if let Some(bt) = type_filter {
            if block.block_type() != bt {
                continue;
            }
        }

        if let Some(ref tb) = turn_blocks {
            if !tb.contains(&id) {
                continue;
            }
        }

        if let Some(ref tb) = tool_blocks {
            if !tb.contains(&id) {
                continue;
            }
        }

        summaries.push(block_summary(id, block));
    }

    if json {
        output::print_json(&summaries)?;
    } else {
        println!(
            "{:<6} {:<10} {:<10} {:<38} {}",
            "Index", "Type", "Time", "UUID", "Summary"
        );
        println!("{}", "-".repeat(100));
        for s in &summaries {
            println!(
                "{:<6} {:<10} {:<10} {:<38} {}",
                s.index, s.block_type, s.timestamp, output::truncate(&s.uuid, 36), s.summary
            );
        }
        println!("\nTotal: {} blocks", summaries.len());
    }

    Ok(())
}

fn run_count(session: &Session, group_by: &str, json: bool) -> Result<()> {
    match group_by {
        "tool" => {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for &id in session.blocks_of_type(BlockType::Tool) {
                if let Block::Tool(t) = session.block(id) {
                    *counts.entry(t.tool_name.clone()).or_default() += 1;
                }
            }

            let mut entries: Vec<_> = counts.into_iter().collect();
            entries.sort_by(|a, b| b.1.cmp(&a.1));

            if json {
                let json_entries: Vec<_> = entries
                    .iter()
                    .map(|(name, count)| serde_json::json!({"tool_name": name, "count": count}))
                    .collect();
                output::print_json(&json_entries)?;
            } else {
                println!("{:<20} {:>6}", "Tool Name", "Count");
                println!("{}", "-".repeat(28));
                for (name, count) in &entries {
                    println!("{:<20} {:>6}", name, count);
                }
            }
        }
        _ => {
            // Default: group by type
            let types = [
                BlockType::User,
                BlockType::Assistant,
                BlockType::Tool,
                BlockType::System,
            ];

            if json {
                let json_entries: Vec<_> = types
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "type": t.to_string(),
                            "count": session.blocks_of_type(*t).len()
                        })
                    })
                    .collect();
                output::print_json(&json_entries)?;
            } else {
                println!("{:<12} {:>6}", "Type", "Count");
                println!("{}", "-".repeat(20));
                let mut total = 0;
                for t in &types {
                    let count = session.blocks_of_type(*t).len();
                    total += count;
                    println!("{:<12} {:>6}", t, count);
                }
                println!("{}", "-".repeat(20));
                println!("{:<12} {:>6}", "Total", total);
            }
        }
    }

    Ok(())
}

fn run_show(session: &Session, uuid_prefix: &str, json: bool, raw: bool) -> Result<()> {
    // Find block by UUID prefix match
    let matching: Vec<_> = session
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.uuid().starts_with(uuid_prefix))
        .collect();

    match matching.len() {
        0 => {
            eprintln!("No block found matching UUID prefix: {uuid_prefix}");
            std::process::exit(1);
        }
        1 => {
            let (id, block) = matching[0];
            if json {
                output::print_json_value(block)?;
            } else if raw {
                // Show raw JSONL lines from provenance
                if let Some(refs) = session.provenance.get(&id) {
                    let file_content = std::fs::read_to_string(&session.file_path)?;
                    let lines: Vec<&str> = file_content.lines().collect();
                    for line_ref in refs {
                        if line_ref.line_number > 0 && line_ref.line_number <= lines.len() {
                            println!("{}", lines[line_ref.line_number - 1]);
                        }
                    }
                }
            } else {
                println!("{}", serde_json::to_string_pretty(block)?);
            }
        }
        n => {
            eprintln!("Ambiguous UUID prefix '{uuid_prefix}' matches {n} blocks:");
            for (_, block) in &matching {
                eprintln!("  {}", block.uuid());
            }
            std::process::exit(1);
        }
    }

    Ok(())
}
