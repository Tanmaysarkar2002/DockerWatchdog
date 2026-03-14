use docker_watchdog::utils::{short_id, truncate_logs};

// ─── short_id tests ───

#[test]
fn test_short_id_long_hash() {
    let full = "abc123def456789abcdef";
    assert_eq!(short_id(full), "abc123def456");
}

#[test]
fn test_short_id_exactly_12() {
    let full = "abc123def456";
    assert_eq!(short_id(full), "abc123def456");
}

#[test]
fn test_short_id_shorter_than_12() {
    let full = "abc123";
    assert_eq!(short_id(full), "abc123");
}

#[test]
fn test_short_id_empty() {
    assert_eq!(short_id(""), "");
}

// ─── truncate_logs tests ───

#[test]
fn test_truncate_logs_within_limit() {
    let logs = "line1\nline2\nline3";
    let result = truncate_logs(logs, 10);
    assert_eq!(result, logs);
}

#[test]
fn test_truncate_logs_over_limit() {
    let logs = "line1\nline2\nline3\nline4\nline5";
    let result = truncate_logs(logs, 3);
    // Should keep last 3 lines and show truncation notice
    assert!(result.contains("2 lines truncated"));
    assert!(result.contains("line3"));
    assert!(result.contains("line4"));
    assert!(result.contains("line5"));
    assert!(!result.contains("line1"));
}

#[test]
fn test_truncate_logs_exact_limit() {
    let logs = "line1\nline2\nline3";
    let result = truncate_logs(logs, 3);
    assert_eq!(result, logs);
}

#[test]
fn test_truncate_logs_single_line_over() {
    let logs = "line1\nline2";
    let result = truncate_logs(logs, 1);
    assert!(result.contains("1 lines truncated"));
    assert!(result.contains("line2"));
}

#[test]
fn test_truncate_logs_empty() {
    let result = truncate_logs("", 10);
    assert_eq!(result, "");
}
