/// Formats a container ID to its short form (first 12 characters).
pub fn short_id(full_id: &str) -> &str {
    if full_id.len() > 12 {
        &full_id[..12]
    } else {
        full_id
    }
}

/// Truncates a log string to a maximum number of lines.
pub fn truncate_logs(logs: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = logs.lines().collect();
    if lines.len() > max_lines {
        let truncated = &lines[lines.len() - max_lines..];
        format!(
            "... ({} lines truncated) ...\n{}",
            lines.len() - max_lines,
            truncated.join("\n")
        )
    } else {
        logs.to_string()
    }
}
