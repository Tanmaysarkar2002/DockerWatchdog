use docker_watchdog::config::AppConfig;

#[test]
fn test_default_config_values() {
    let config = AppConfig::default();

    assert_eq!(config.log_tail_lines, 50);
    assert_eq!(config.log_level, "info");
    assert!(config.colored_output);
    assert!(config.auto_restart);
    assert_eq!(config.max_restarts, 3);
    assert_eq!(config.restart_delay_secs, 5);
    assert!(config.ignore_containers.is_empty());
    assert_eq!(config.cooldown_secs, 300);
    assert!(config.restart_backoff);
    assert_eq!(config.heartbeat_secs, 300);
    assert_eq!(config.event_timeout_secs, 600);
    assert!(!config.dry_run);
    assert!(config.startup_scan);
    assert_eq!(config.dedup_window_secs, 5);
}

#[test]
fn test_from_env_overrides_log_tail() {
    unsafe { std::env::set_var("WATCHDOG_LOG_TAIL", "100") };
    let config = AppConfig::from_env();
    assert_eq!(config.log_tail_lines, 100);
    unsafe { std::env::remove_var("WATCHDOG_LOG_TAIL") };
}

#[test]
fn test_from_env_overrides_log_level() {
    unsafe { std::env::set_var("WATCHDOG_LOG_LEVEL", "debug") };
    let config = AppConfig::from_env();
    assert_eq!(config.log_level, "debug");
    unsafe { std::env::remove_var("WATCHDOG_LOG_LEVEL") };
}

#[test]
fn test_from_env_auto_restart_true() {
    unsafe { std::env::set_var("WATCHDOG_AUTO_RESTART", "true") };
    let config = AppConfig::from_env();
    assert!(config.auto_restart);
    unsafe { std::env::remove_var("WATCHDOG_AUTO_RESTART") };
}

#[test]
fn test_from_env_auto_restart_1() {
    unsafe { std::env::set_var("WATCHDOG_AUTO_RESTART", "1") };
    let config = AppConfig::from_env();
    assert!(config.auto_restart);
    unsafe { std::env::remove_var("WATCHDOG_AUTO_RESTART") };
}

#[test]
fn test_from_env_invalid_log_tail_ignored() {
    unsafe { std::env::set_var("WATCHDOG_LOG_TAIL", "not_a_number") };
    let config = AppConfig::from_env();
    assert_eq!(config.log_tail_lines, 50);
    unsafe { std::env::remove_var("WATCHDOG_LOG_TAIL") };
}

#[test]
fn test_from_env_max_restarts() {
    unsafe { std::env::set_var("WATCHDOG_MAX_RESTARTS", "10") };
    let config = AppConfig::from_env();
    assert_eq!(config.max_restarts, 10);
    unsafe { std::env::remove_var("WATCHDOG_MAX_RESTARTS") };
}

#[test]
fn test_from_env_no_color() {
    unsafe { std::env::set_var("WATCHDOG_NO_COLOR", "true") };
    let config = AppConfig::from_env();
    assert!(!config.colored_output);
    unsafe { std::env::remove_var("WATCHDOG_NO_COLOR") };
}

#[test]
fn test_from_env_ignore_containers() {
    unsafe { std::env::set_var("WATCHDOG_IGNORE_CONTAINERS", "redis, nginx, postgres") };
    let config = AppConfig::from_env();
    assert_eq!(config.ignore_containers, vec!["redis", "nginx", "postgres"]);
    unsafe { std::env::remove_var("WATCHDOG_IGNORE_CONTAINERS") };
}

#[test]
fn test_from_env_ignore_containers_empty() {
    unsafe { std::env::set_var("WATCHDOG_IGNORE_CONTAINERS", "") };
    let config = AppConfig::from_env();
    assert!(config.ignore_containers.is_empty());
    unsafe { std::env::remove_var("WATCHDOG_IGNORE_CONTAINERS") };
}

#[test]
fn test_from_env_cooldown_secs() {
    unsafe { std::env::set_var("WATCHDOG_COOLDOWN_SECS", "120") };
    let config = AppConfig::from_env();
    assert_eq!(config.cooldown_secs, 120);
    unsafe { std::env::remove_var("WATCHDOG_COOLDOWN_SECS") };
}

#[test]
fn test_from_env_heartbeat_and_timeout() {
    unsafe { std::env::set_var("WATCHDOG_HEARTBEAT_SECS", "60") };
    unsafe { std::env::set_var("WATCHDOG_EVENT_TIMEOUT_SECS", "120") };
    let config = AppConfig::from_env();
    assert_eq!(config.heartbeat_secs, 60);
    assert_eq!(config.event_timeout_secs, 120);
    unsafe { std::env::remove_var("WATCHDOG_HEARTBEAT_SECS") };
    unsafe { std::env::remove_var("WATCHDOG_EVENT_TIMEOUT_SECS") };
}

#[test]
fn test_validation_caps_max_restarts() {
    unsafe { std::env::set_var("WATCHDOG_MAX_RESTARTS", "999") };
    let config = AppConfig::from_env();
    assert_eq!(config.max_restarts, 100);
    unsafe { std::env::remove_var("WATCHDOG_MAX_RESTARTS") };
}

#[test]
fn test_validation_caps_restart_delay() {
    unsafe { std::env::set_var("WATCHDOG_RESTART_DELAY", "9999") };
    let config = AppConfig::from_env();
    assert_eq!(config.restart_delay_secs, 3600);
    unsafe { std::env::remove_var("WATCHDOG_RESTART_DELAY") };
}

#[test]
fn test_validation_floors_heartbeat() {
    unsafe { std::env::set_var("WATCHDOG_HEARTBEAT_SECS", "1") };
    let config = AppConfig::from_env();
    assert_eq!(config.heartbeat_secs, 10);
    unsafe { std::env::remove_var("WATCHDOG_HEARTBEAT_SECS") };
}

#[test]
fn test_validation_floors_event_timeout() {
    unsafe { std::env::set_var("WATCHDOG_EVENT_TIMEOUT_SECS", "5") };
    let config = AppConfig::from_env();
    assert_eq!(config.event_timeout_secs, 30);
    unsafe { std::env::remove_var("WATCHDOG_EVENT_TIMEOUT_SECS") };
}

#[test]
fn test_from_env_restart_backoff() {
    unsafe { std::env::set_var("WATCHDOG_RESTART_BACKOFF", "false") };
    let config = AppConfig::from_env();
    assert!(!config.restart_backoff);
    unsafe { std::env::remove_var("WATCHDOG_RESTART_BACKOFF") };
}

#[test]
fn test_from_env_dry_run() {
    unsafe { std::env::set_var("WATCHDOG_DRY_RUN", "true") };
    let config = AppConfig::from_env();
    assert!(config.dry_run);
    unsafe { std::env::remove_var("WATCHDOG_DRY_RUN") };
}

#[test]
fn test_from_env_startup_scan_disabled() {
    unsafe { std::env::set_var("WATCHDOG_STARTUP_SCAN", "false") };
    let config = AppConfig::from_env();
    // "false" doesn't match "1" or "true", so env_bool returns Some(false)
    assert!(!config.startup_scan);
    unsafe { std::env::remove_var("WATCHDOG_STARTUP_SCAN") };
}

#[test]
fn test_from_env_dedup_window() {
    unsafe { std::env::set_var("WATCHDOG_DEDUP_WINDOW_SECS", "10") };
    let config = AppConfig::from_env();
    assert_eq!(config.dedup_window_secs, 10);
    unsafe { std::env::remove_var("WATCHDOG_DEDUP_WINDOW_SECS") };
}
