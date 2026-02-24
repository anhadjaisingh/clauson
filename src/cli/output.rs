use serde::Serialize;

/// Print as JSON array to stdout.
pub fn print_json<T: Serialize>(items: &[T]) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(items)?);
    Ok(())
}

/// Print a single item as JSON.
pub fn print_json_value<T: Serialize>(item: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(item)?);
    Ok(())
}

/// Truncate a string to max_len, appending "..." if truncated.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format a number with comma separators.
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

pub enum Align {
    Left,
    Right,
}

pub struct Column {
    name: String,
    align: Align,
}

impl Column {
    pub fn left(name: &str) -> Self {
        Column {
            name: name.to_string(),
            align: Align::Left,
        }
    }

    pub fn right(name: &str) -> Self {
        Column {
            name: name.to_string(),
            align: Align::Right,
        }
    }
}

pub struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(columns: Vec<Column>) -> Self {
        Table {
            columns,
            rows: vec![],
        }
    }

    pub fn add_row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    fn widths(&self) -> Vec<usize> {
        self.columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let header_w = col.name.len();
                let max_cell = self
                    .rows
                    .iter()
                    .map(|row| row.get(i).map_or(0, |c| c.len()))
                    .max()
                    .unwrap_or(0);
                header_w.max(max_cell)
            })
            .collect()
    }

    fn format_row(&self, cells: &[String], widths: &[usize]) -> String {
        self.columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let w = widths[i];
                let val = cells.get(i).map_or("", |s| s.as_str());
                match col.align {
                    Align::Left => format!("{val:<w$}"),
                    Align::Right => format!("{val:>w$}"),
                }
            })
            .collect::<Vec<_>>()
            .join("  ")
    }

    pub fn print(&self) {
        let widths = self.widths();
        let header: Vec<String> = self.columns.iter().map(|c| c.name.clone()).collect();
        println!("{}", self.format_row(&header, &widths));
        let total_width: usize = widths.iter().sum::<usize>() + (widths.len().saturating_sub(1)) * 2;
        println!("{}", "-".repeat(total_width));
        for row in &self.rows {
            println!("{}", self.format_row(row, &widths));
        }
    }

    pub fn print_with_total(&self, footer: &str) {
        self.print();
        println!("\n{footer}");
    }
}
