use std::borrow::Cow;

pub fn short_id(full_id: &str) -> &str {
    if full_id.len() > 12 {
        &full_id[..12]
    } else {
        full_id
    }
}

pub fn truncate_logs(logs: &str, max_lines: usize) -> Cow<'_, str> {
    let lines: Vec<&str> = logs.lines().collect();
    if lines.len() > max_lines {
        let truncated = &lines[lines.len() - max_lines..];
        Cow::Owned(format!(
            "... ({} lines truncated) ...\n{}",
            lines.len() - max_lines,
            truncated.join("\n")
        ))
    } else {
        Cow::Borrowed(logs)
    }
}
