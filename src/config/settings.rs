use tracing::warn;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub log_tail_lines: u64,
    pub log_level: String,
    pub colored_output: bool,
    pub auto_restart: bool,
    pub max_restarts: u32,
    pub restart_delay_secs: u64,
    pub compose_project: Option<String>,
    pub ignore_containers: Vec<String>,
    pub cooldown_secs: u64,
    pub restart_backoff: bool,
    pub heartbeat_secs: u64,
    pub event_timeout_secs: u64,
    pub dry_run: bool,
    pub startup_scan: bool,
    pub dedup_window_secs: u64,
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
            ignore_containers: Vec::new(),
            cooldown_secs: 300,
            restart_backoff: true,
            heartbeat_secs: 300,
            event_timeout_secs: 600,
            dry_run: false,
            startup_scan: true,
            dedup_window_secs: 5,
        }
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, target: &mut T) {
    if let Ok(val) = std::env::var(key) {
        if let Ok(parsed) = val.parse() {
            *target = parsed;
        }
    }
}

fn env_bool(key: &str) -> Option<bool> {
    std::env::var(key).ok().map(|val| val == "1" || val.eq_ignore_ascii_case("true"))
}

impl AppConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        env_parse("WATCHDOG_LOG_TAIL", &mut config.log_tail_lines);
        env_parse("WATCHDOG_LOG_LEVEL", &mut config.log_level);
        env_parse("WATCHDOG_MAX_RESTARTS", &mut config.max_restarts);
        env_parse("WATCHDOG_RESTART_DELAY", &mut config.restart_delay_secs);
        env_parse("WATCHDOG_COOLDOWN_SECS", &mut config.cooldown_secs);
        env_parse("WATCHDOG_HEARTBEAT_SECS", &mut config.heartbeat_secs);
        env_parse("WATCHDOG_EVENT_TIMEOUT_SECS", &mut config.event_timeout_secs);
        env_parse("WATCHDOG_DEDUP_WINDOW_SECS", &mut config.dedup_window_secs);

        if let Some(true) = env_bool("WATCHDOG_NO_COLOR") {
            config.colored_output = false;
        }
        if let Some(val) = env_bool("WATCHDOG_AUTO_RESTART") {
            config.auto_restart = val;
        }
        if let Some(val) = env_bool("WATCHDOG_RESTART_BACKOFF") {
            config.restart_backoff = val;
        }
        if let Some(val) = env_bool("WATCHDOG_DRY_RUN") {
            config.dry_run = val;
        }
        if let Some(val) = env_bool("WATCHDOG_STARTUP_SCAN") {
            config.startup_scan = val;
        }

        if let Ok(val) = std::env::var("WATCHDOG_COMPOSE_PROJECT") {
            if !val.is_empty() {
                config.compose_project = Some(val);
            }
        }

        if let Ok(val) = std::env::var("WATCHDOG_IGNORE_CONTAINERS") {
            config.ignore_containers = val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        config.validate();
        config
    }

    fn validate(&mut self) {
        if self.max_restarts > 100 {
            warn!("WATCHDOG_MAX_RESTARTS={} is unusually high, capping at 100.", self.max_restarts);
            self.max_restarts = 100;
        }
        if self.restart_delay_secs > 3600 {
            warn!("WATCHDOG_RESTART_DELAY={}s exceeds 1 hour, capping at 3600.", self.restart_delay_secs);
            self.restart_delay_secs = 3600;
        }
        if self.heartbeat_secs < 10 {
            warn!("WATCHDOG_HEARTBEAT_SECS={} is too low, setting to 10.", self.heartbeat_secs);
            self.heartbeat_secs = 10;
        }
        if self.event_timeout_secs < 30 {
            warn!("WATCHDOG_EVENT_TIMEOUT_SECS={} is too low, setting to 30.", self.event_timeout_secs);
            self.event_timeout_secs = 30;
        }
        if self.log_tail_lines > 10000 {
            warn!("WATCHDOG_LOG_TAIL={} is very high, capping at 10000.", self.log_tail_lines);
            self.log_tail_lines = 10000;
        }
        if self.dry_run {
            warn!("[DRY-RUN] Dry-run mode enabled — no containers will actually be restarted.");
        }
    }
}
