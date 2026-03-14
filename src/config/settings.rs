/// Application-level configuration for docker-watchdog.
///
/// This struct holds all tunable parameters. In the future,
/// this can be loaded from a TOML/YAML config file.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Number of tail log lines to fetch when a container dies.
    pub log_tail_lines: u64,
    /// Log level filter (e.g., "info", "debug", "warn").
    pub log_level: String,
    /// Whether to enable colored terminal output.
    pub colored_output: bool,
    /// Whether to auto-restart containers that exit with non-zero codes.
    pub auto_restart: bool,
    /// Maximum number of restart attempts per container before giving up.
    pub max_restarts: u32,
    /// Delay in seconds before attempting a restart.
    pub restart_delay_secs: u64,
    /// If set, only monitor containers from this compose project.
    /// Matches the `com.docker.compose.project` label.
    pub compose_project: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            log_tail_lines: 50,
            log_level: "info".to_string(),
            colored_output: true,
            auto_restart: true,
            max_restarts: 3,
            restart_delay_secs: 5,
            compose_project: None,
        }
    }
}

impl AppConfig {
    /// Creates a new AppConfig from environment variables,
    /// falling back to defaults for any missing values.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("WATCHDOG_LOG_TAIL") {
            if let Ok(n) = val.parse::<u64>() {
                config.log_tail_lines = n;
            }
        }

        if let Ok(val) = std::env::var("WATCHDOG_LOG_LEVEL") {
            config.log_level = val;
        }

        if let Ok(val) = std::env::var("WATCHDOG_NO_COLOR") {
            if val == "1" || val.to_lowercase() == "true" {
                config.colored_output = false;
            }
        }

        if let Ok(val) = std::env::var("WATCHDOG_AUTO_RESTART") {
            if val == "1" || val.to_lowercase() == "true" {
                config.auto_restart = true;
            }
        }

        if let Ok(val) = std::env::var("WATCHDOG_MAX_RESTARTS") {
            if let Ok(n) = val.parse::<u32>() {
                config.max_restarts = n;
            }
        }

        if let Ok(val) = std::env::var("WATCHDOG_RESTART_DELAY") {
            if let Ok(n) = val.parse::<u64>() {
                config.restart_delay_secs = n;
            }
        }

        if let Ok(val) = std::env::var("WATCHDOG_COMPOSE_PROJECT") {
            if !val.is_empty() {
                config.compose_project = Some(val);
            }
        }

        config
    }
}
