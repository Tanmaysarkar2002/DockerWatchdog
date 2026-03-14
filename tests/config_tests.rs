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
}

#[test]
fn test_from_env_overrides_log_tail() {
    // SAFETY: Tests run with --test-threads=1 to avoid env var races.
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
    assert_eq!(config.log_tail_lines, 50); // Should fall back to default
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
