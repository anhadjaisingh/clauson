use anyhow::Result;
use clap::Subcommand;

use clauson::model::block::{Block, BlockInfo};
use clauson::model::session::Session;

use super::output;

#[derive(Subcommand)]
pub enum TurnsAction {
    /// List all turns
    List,
    /// Show details of a specific turn
    Show {
        /// Turn number (1-indexed)
        number: usize,
    },
}

pub fn run(session: &Session, action: Option<&TurnsAction>, json: bool) -> Result<()> {
    match action {
        None | Some(TurnsAction::List) => run_list(session, json),
        Some(TurnsAction::Show { number }) => run_show(session, *number, json),
    }
}

fn run_list(session: &Session, json: bool) -> Result<()> {
    let turns = session.turns();

    if json {
        let json_turns: Vec<_> = turns
            .iter()
            .map(|turn| {
                let user_prompt = if let Block::User(u) = session.block(turn.user_block) {
                    u.content.as_deref().unwrap_or("(no content)")
                } else {
                    "(unknown)"
                };
                let duration = turn
                    .duration_ms
                    .map(|d| format!("{:.1}s", d as f64 / 1000.0));

                serde_json::json!({
                    "turn": turn.index + 1,
                    "timestamp": session.block(turn.user_block).timestamp().format("%Y-%m-%d %H:%M:%S").to_string(),
                    "blocks": turn.all_blocks.len(),
                    "tools": turn.tool_blocks.len(),
                    "total_tokens": turn.total_tokens.total(),
                    "duration": duration,
                    "user_prompt": output::truncate(user_prompt, 80),
                })
            })
            .collect();
        output::print_json(&json_turns)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::right("Turn"),
            output::Column::right("Timestamp"),
            output::Column::right("Blocks"),
            output::Column::right("Tools"),
            output::Column::right("Tokens"),
            output::Column::right("Duration"),
            output::Column::left("User Prompt"),
        ]);

        for turn in &turns {
            let user_prompt = if let Block::User(u) = session.block(turn.user_block) {
                u.content.as_deref().unwrap_or("(no content)")
            } else {
                "(unknown)"
            };
            let duration = turn
                .duration_ms
                .map(|d| format!("{:.1}s", d as f64 / 1000.0))
                .unwrap_or_default();

            table.add_row(vec![
                (turn.index + 1).to_string(),
                session
                    .block(turn.user_block)
                    .timestamp()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                turn.all_blocks.len().to_string(),
                turn.tool_blocks.len().to_string(),
                output::format_number(turn.total_tokens.total()),
                duration,
                output::truncate(user_prompt, 40),
            ]);
        }

        table.print_with_total(&format!("Total: {} turns", turns.len()));
    }

    Ok(())
}

fn run_show(session: &Session, number: usize, json: bool) -> Result<()> {
    let turns = session.turns();
    if number == 0 || number > turns.len() {
        eprintln!("Turn {number} not found (session has {} turns)", turns.len());
        std::process::exit(1);
    }

    let turn = &turns[number - 1];

    if json {
        let blocks: Vec<_> = turn
            .all_blocks
            .iter()
            .map(|&id| session.block(id))
            .collect();
        output::print_json(&blocks)?;
    } else {
        let user_prompt = if let Block::User(u) = session.block(turn.user_block) {
            u.content.as_deref().unwrap_or("(no content)")
        } else {
            "(unknown)"
        };

        println!("Turn {} Details", number);
        println!("{}", "─".repeat(60));
        println!("User prompt: {}", output::truncate(user_prompt, 200));
        println!(
            "Timestamp:   {}",
            session
                .block(turn.user_block)
                .timestamp()
                .format("%Y-%m-%d %H:%M:%S UTC")
        );
        if let Some(d) = turn.duration_ms {
            println!("Duration:    {:.1}s", d as f64 / 1000.0);
        }
        println!(
            "Tokens:      {} total",
            output::format_number(turn.total_tokens.total())
        );
        println!();
        println!("Blocks ({}):", turn.all_blocks.len());
        let mut table = output::Table::new(vec![
            output::Column::left("Index"),
            output::Column::left("Type"),
            output::Column::left("Time"),
            output::Column::left("Summary"),
        ]);

        for &id in &turn.all_blocks {
            let block = session.block(id);
            let summary = match block {
                Block::User(u) => output::truncate(
                    u.content.as_deref().unwrap_or("(no content)"),
                    50,
                ),
                Block::Assistant(a) => {
                    let mut parts = vec![];
                    if let Some(c) = &a.content {
                        parts.push(output::truncate(c, 40));
                    }
                    if !a.tool_calls.is_empty() {
                        parts.push(format!("[{} tools]", a.tool_calls.len()));
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

            table.add_row(vec![
                id.to_string(),
                block.block_type().to_string(),
                block.timestamp().format("%H:%M:%S").to_string(),
                summary,
            ]);
        }
        table.print();
    }

    Ok(())
}
