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
