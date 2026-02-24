mod cli;

use clap::Parser;
use cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // tool-events reads its own sidecar file, not the session JSONL
    if let cli::Command::ToolEvents { action } = &cli.command {
        return cli::tool_events::run(&cli.file, action.as_ref(), cli.json);
    }

    let session = clauson::parser::parse_session(&cli.file)?;

    match &cli.command {
        cli::Command::Blocks { action } => {
            cli::blocks::run(&session, action.as_ref(), cli.json, cli.raw)?;
        }
        cli::Command::Tools { action } => {
            cli::tools::run(&session, action.as_ref(), cli.json)?;
        }
        cli::Command::Turns { action } => {
            cli::turns::run(&session, action.as_ref(), cli.json)?;
        }
        cli::Command::Stats { action } => {
            cli::stats::run(&session, action.as_ref(), cli.json)?;
        }
        cli::Command::ToolEvents { .. } => unreachable!(),
    }
    Ok(())
}
