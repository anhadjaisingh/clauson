use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod blocks;
pub mod output;
pub mod stats;
pub mod tool_events;
pub mod tools;
pub mod turns;

#[derive(Parser)]
#[command(name = "clauson", about = "Claude session JSONL analyzer")]
pub struct Cli {
    /// Path to the JSONL session file
    pub file: PathBuf,

    #[command(subcommand)]
    pub command: Command,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Show raw JSONL lines instead of parsed blocks
    #[arg(long, global = true)]
    pub raw: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Query and filter blocks
    Blocks {
        #[command(subcommand)]
        action: Option<blocks::BlocksAction>,
    },
    /// List and analyze tool usage
    Tools {
        #[command(subcommand)]
        action: Option<tools::ToolsAction>,
    },
    /// View conversation turns
    Turns {
        #[command(subcommand)]
        action: Option<turns::TurnsAction>,
    },
    /// Token, time, and tool-call analytics
    Stats {
        #[command(subcommand)]
        action: Option<stats::StatsAction>,
    },
    /// Analyze tool lifecycle and permission events from sidecar log
    #[command(name = "tool-events")]
    ToolEvents {
        #[command(subcommand)]
        action: Option<tool_events::ToolEventsAction>,
    },
}
