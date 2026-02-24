mod cli;

use clap::Parser;
use cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
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
    }
    Ok(())
}
