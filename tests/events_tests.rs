use docker_watchdog::docker::ContainerEvent;

#[test]
fn test_is_failure_nonzero_exit() {
    let event = ContainerEvent {
        container_id: "abc123".to_string(),
        container_name: "my-app".to_string(),
        action: "die".to_string(),
        exit_code: Some(1),
    };
    assert!(event.is_failure());
}

#[test]
fn test_is_failure_zero_exit() {
    let event = ContainerEvent {
        container_id: "abc123".to_string(),
        container_name: "my-app".to_string(),
        action: "die".to_string(),
        exit_code: Some(0),
    };
    assert!(!event.is_failure());
}

#[test]
fn test_is_failure_oom_killed() {
    let event = ContainerEvent {
        container_id: "abc123".to_string(),
        container_name: "my-app".to_string(),
        action: "die".to_string(),
        exit_code: Some(137),
    };
    assert!(event.is_failure());
}

#[test]
fn test_is_failure_no_exit_code_die_action() {
    let event = ContainerEvent {
        container_id: "abc123".to_string(),
        container_name: "my-app".to_string(),
        action: "die".to_string(),
        exit_code: None,
    };
    assert!(event.is_failure());
}

#[test]
fn test_is_failure_no_exit_code_other_action() {
    let event = ContainerEvent {
        container_id: "abc123".to_string(),
        container_name: "my-app".to_string(),
        action: "stop".to_string(),
        exit_code: None,
    };
    assert!(!event.is_failure());
}

#[test]
fn test_summary_with_exit_code() {
    let event = ContainerEvent {
        container_id: "abc123def456".to_string(),
        container_name: "web-server".to_string(),
        action: "die".to_string(),
        exit_code: Some(1),
    };
    let summary = event.summary();
    assert!(summary.contains("web-server"));
    assert!(summary.contains("abc123def456"));
    assert!(summary.contains("die"));
    assert!(summary.contains("1"));
}

#[test]
fn test_summary_without_exit_code() {
    let event = ContainerEvent {
        container_id: "abc123".to_string(),
        container_name: "db".to_string(),
        action: "die".to_string(),
        exit_code: None,
    };
    let summary = event.summary();
    assert!(summary.contains("unknown"));
}
