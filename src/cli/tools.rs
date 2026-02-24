use anyhow::Result;
use clap::Subcommand;
use std::collections::HashMap;

use clauson::model::block::Block;
use clauson::model::session::Session;
use clauson::model::types::BlockType;

use super::output;

#[derive(Subcommand)]
pub enum ToolsAction {
    /// List unique tools with counts
    List {
        /// Sort by: count (default), name
        #[arg(long, default_value = "count")]
        sort: String,
    },
}

pub fn run(session: &Session, action: Option<&ToolsAction>, json: bool) -> Result<()> {
    let sort = match action {
        Some(ToolsAction::List { sort }) => sort.as_str(),
        None => "count",
    };
    run_list(session, sort, json)
}

fn run_list(session: &Session, sort: &str, json: bool) -> Result<()> {
    let mut tool_stats: HashMap<String, ToolStat> = HashMap::new();

    for &id in session.blocks_of_type(BlockType::Tool) {
        if let Block::Tool(t) = session.block(id) {
            let stat = tool_stats.entry(t.tool_name.clone()).or_insert(ToolStat {
                name: t.tool_name.clone(),
                count: 0,
                errors: 0,
            });
            stat.count += 1;
            if t.is_error {
                stat.errors += 1;
            }
        }
    }

    let mut entries: Vec<ToolStat> = tool_stats.into_values().collect();
    match sort {
        "name" => entries.sort_by(|a, b| a.name.cmp(&b.name)),
        _ => entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name))),
    }

    if json {
        let json_entries: Vec<_> = entries
            .iter()
            .map(|s| {
                serde_json::json!({
                    "tool_name": s.name,
                    "count": s.count,
                    "errors": s.errors,
                })
            })
            .collect();
        output::print_json(&json_entries)?;
    } else {
        let mut table = output::Table::new(vec![
            output::Column::left("Tool Name"),
            output::Column::right("Count"),
            output::Column::right("Errors"),
        ]);
        for s in &entries {
            table.add_row(vec![
                s.name.clone(),
                s.count.to_string(),
                s.errors.to_string(),
            ]);
        }
        table.print_with_total(&format!("Total: {} unique tools", entries.len()));
    }

    Ok(())
}

struct ToolStat {
    name: String,
    count: usize,
    errors: usize,
}
