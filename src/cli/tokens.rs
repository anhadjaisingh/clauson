use anyhow::Result;
use clap::Subcommand;

use clauson::model::block::{Block, BlockInfo};
use clauson::model::session::Session;
use clauson::model::types::{BlockType, TokenUsage};

use super::output;

#[derive(Subcommand)]
pub enum TokensAction {
    /// Show aggregate token summary
    Summary,
    /// Show token usage per turn
    ByTurn,
}

pub fn run(session: &Session, action: Option<&TokensAction>, json: bool) -> Result<()> {
    match action {
        None | Some(TokensAction::Summary) => run_summary(session, json),
        Some(TokensAction::ByTurn) => run_by_turn(session, json),
    }
}

fn run_summary(session: &Session, json: bool) -> Result<()> {
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
        println!("{}", "─".repeat(40));
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
        println!("{}", "─".repeat(40));
        println!(
            "  Total:                   {:>12}",
            output::format_number(total.total())
        );
    }

    Ok(())
}

fn run_by_turn(session: &Session, json: bool) -> Result<()> {
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

                serde_json::json!({
                    "turn": turn.index + 1,
                    "user_prompt": output::truncate(user_prompt, 100),
                    "input_tokens": turn.total_tokens.input_tokens,
                    "output_tokens": turn.total_tokens.output_tokens,
                    "cache_creation_input_tokens": turn.total_tokens.cache_creation_input_tokens,
                    "cache_read_input_tokens": turn.total_tokens.cache_read_input_tokens,
                    "total": turn.total_tokens.total(),
                    "duration_ms": turn.duration_ms,
                })
            })
            .collect();
        output::print_json(&json_turns)?;
    } else {
        println!(
            "{:>5} {:>10} {:>14} {:>12} {:>10} {:>10}  {}",
            "Turn", "Input", "Cache Create", "Cache Read", "Output", "Total", "User Prompt"
        );
        println!("{}", "-".repeat(100));

        for turn in &turns {
            let user_prompt = if let Block::User(u) = session.block(turn.user_block) {
                u.content.as_deref().unwrap_or("(no content)")
            } else {
                "(unknown)"
            };

            println!(
                "{:>5} {:>10} {:>14} {:>12} {:>10} {:>10}  {}",
                turn.index + 1,
                output::format_number(turn.total_tokens.input_tokens),
                output::format_number(turn.total_tokens.cache_creation_input_tokens),
                output::format_number(turn.total_tokens.cache_read_input_tokens),
                output::format_number(turn.total_tokens.output_tokens),
                output::format_number(turn.total_tokens.total()),
                output::truncate(user_prompt, 40),
            );
        }
    }

    Ok(())
}
