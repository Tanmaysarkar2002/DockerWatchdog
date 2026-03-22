use std::collections::HashMap;
use bollard::models::EventMessageTypeEnum;
use bollard::query_parameters::{EventsOptions, ListContainersOptions};
use futures_util::StreamExt;
use tokio::time::{Duration, Instant, interval, timeout};
use tracing::{debug, error, info, warn};

use crate::config::AppConfig;
use crate::docker::{self, ContainerEvent};
use crate::logger;
use crate::notifier::Notifier;
use crate::restarter::RestartTracker;
use crate::utils;

const MAX_RECONNECT_DELAY_SECS: u64 = 60;
const STALE_CLEANUP_SECS: u64 = 3600;

struct CrashMetrics {
    crashes: HashMap<String, u32>,
}

impl CrashMetrics {
    fn new() -> Self {
        Self { crashes: HashMap::new() }
    }

    fn record_crash(&mut self, container_name: &str) {
        *self.crashes.entry(container_name.to_string()).or_insert(0) += 1;
    }

    fn log_summary(&self) {
        if self.crashes.is_empty() {
            info!("[METRICS] No container crashes recorded.");
            return;
        }
        let total: u32 = self.crashes.values().sum();
        let mut entries: Vec<_> = self.crashes.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        let top: Vec<String> = entries.iter()
            .take(10)
            .map(|(name, count)| format!("{}={}", name, count))
            .collect();
        info!("[METRICS] Total crashes: {}. Top containers: [{}]", total, top.join(", "));
    }
}

struct EventDedup {
    recent: HashMap<(String, String), Instant>,
    window: Duration,
}

impl EventDedup {
    fn new(window_secs: u64) -> Self {
        Self {
            recent: HashMap::new(),
            window: Duration::from_secs(window_secs),
        }
    }

    fn is_duplicate(&mut self, container_id: &str, action: &str) -> bool {
        if self.window.is_zero() {
            return false;
        }
        let key = (container_id.to_string(), action.to_string());
        let now = Instant::now();
        if let Some(last) = self.recent.get(&key) {
            if now.duration_since(*last) < self.window {
                return true;
            }
        }
        self.recent.insert(key, now);
        false
    }

    fn cleanup(&mut self) {
        let window = self.window;
        self.recent.retain(|_, last| last.elapsed() < window + Duration::from_secs(10));
    }
}

/// Per-container watchdog label overrides (watchdog.enabled, watchdog.max-restarts).
struct LabelOverrides {
    pub enabled: bool,
    pub max_restarts: Option<u32>,
}

impl LabelOverrides {
    fn from_attrs(attrs: Option<&HashMap<String, String>>) -> Self {
        let attrs = match attrs {
            Some(a) => a,
            None => return Self { enabled: true, max_restarts: None },
        };

        let enabled = attrs
            .get("watchdog.enabled")
            .map_or(true, |v| v != "false" && v != "0");

        let max_restarts = attrs
            .get("watchdog.max-restarts")
            .and_then(|v| v.parse::<u32>().ok());

        Self { enabled, max_restarts }
    }
}

pub async fn run(
    config: &AppConfig,
    notifiers: &[Box<dyn Notifier>],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = RestartTracker::new(
        config.max_restarts,
        config.restart_delay_secs,
        config.restart_backoff,
        config.cooldown_secs,
        config.dry_run,
    );
    let mut metrics = CrashMetrics::new();
    let mut attempts: u32 = 0;

    loop {
        match docker::connect().await {
            Ok(docker) => {
                if attempts > 0 {
                    info!("Reconnected to Docker daemon after {} attempts.", attempts);
                }
                attempts = 0;

                let effective_config = auto_detect_compose(config, &docker).await;

                if effective_config.startup_scan {
                    scan_crashed_containers(&docker, &effective_config).await;
                }

                if let Err(e) =
                    event_loop(&docker, &effective_config, notifiers, &mut tracker, &mut metrics).await
                {
                    error!("Event loop error: {}. Reconnecting...", e);
                } else {
                    warn!("Docker event stream ended. Reconnecting...");
                }
            }
            Err(e) => {
                attempts += 1;
                let delay = (1u64 << attempts.min(6)).min(MAX_RECONNECT_DELAY_SECS);
                error!("Cannot reach Docker daemon: {} (retry in {}s)", e, delay);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
        }
        attempts += 1;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn scan_crashed_containers(docker: &bollard::Docker, config: &AppConfig) {
    info!("[STARTUP SCAN] Checking for containers that crashed while watchdog was offline...");

    let opts = ListContainersOptions {
        all: true,
        filters: {
            let mut f = HashMap::new();
            f.insert("status".to_string(), vec!["exited".to_string()]);
            if let Some(ref project) = config.compose_project {
                f.insert(
                    "label".to_string(),
                    vec![format!("com.docker.compose.project={}", project)],
                );
            }
            Some(f)
        },
        ..Default::default()
    };

    let containers = match docker.list_containers(Some(opts)).await {
        Ok(c) => c,
        Err(e) => {
            warn!("[STARTUP SCAN] Failed to list containers: {}", e);
            return;
        }
    };

    let mut crashed_count = 0u32;
    for container in &containers {
        let name = container.names.as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/'))
            .unwrap_or("unknown");

        if config.ignore_containers.iter().any(|ignored| ignored == name) {
            continue;
        }

        let status = container.status.as_deref().unwrap_or("");
        let id = container.id.as_deref().unwrap_or("unknown");
        let short = utils::short_id(id);

        // Parse exit code from status like "Exited (1) 5 minutes ago"
        if let Some(code) = extract_exit_code_from_status(status) {
            if code != 0 {
                crashed_count += 1;
                warn!(
                    "[STARTUP SCAN] Container '{}' ({}) exited with code {} — {}",
                    name, short, code, status
                );
            }
        }
    }

    if crashed_count == 0 {
        info!("[STARTUP SCAN] No crashed containers found.");
    } else {
        warn!(
            "[STARTUP SCAN] Found {} crashed container(s). Enable auto-restart to recover them.",
            crashed_count
        );
    }
}

fn extract_exit_code_from_status(status: &str) -> Option<i32> {
    let start = status.find('(')? + 1;
    let end = status.find(')')?;
    status[start..end].parse().ok()
}

fn build_event_options(config: &AppConfig) -> EventsOptions {
    let mut filters = HashMap::new();
    filters.insert("type".to_string(), vec!["container".to_string()]);
    filters.insert(
        "event".to_string(),
        vec!["die".to_string(), "start".to_string(), "health_status".to_string()],
    );
    if let Some(ref project) = config.compose_project {
        filters.insert(
            "label".to_string(),
            vec![format!("com.docker.compose.project={}", project)],
        );
    }

    EventsOptions {
        filters: Some(filters),
        ..Default::default()
    }
}

async fn event_loop(
    docker: &bollard::Docker,
    config: &AppConfig,
    notifiers: &[Box<dyn Notifier>],
    tracker: &mut RestartTracker,
    metrics: &mut CrashMetrics,
) -> Result<(), Box<dyn std::error::Error>> {
    let opts = build_event_options(config);
    let mut events = docker.events(Some(opts));
    let mut heartbeat = interval(Duration::from_secs(config.heartbeat_secs));
    let mut cleanup_timer = interval(Duration::from_secs(STALE_CLEANUP_SECS));
    let mut dedup = EventDedup::new(config.dedup_window_secs);
    heartbeat.tick().await;
    cleanup_timer.tick().await;

    info!("[WATCHING] Listening for Docker container events...");
    if config.auto_restart {
        info!(
            "Auto-restart enabled (max {} attempts, {}s base delay, backoff={})",
            config.max_restarts, config.restart_delay_secs, config.restart_backoff
        );
    }
    if config.dry_run {
        warn!("[DRY-RUN] Running in dry-run mode — restarts will be logged but not executed.");
    }
    if let Some(ref project) = config.compose_project {
        info!("Filtering to compose project: '{}'", project);
    }
    if !config.ignore_containers.is_empty() {
        info!("Ignoring containers: {:?}", config.ignore_containers);
    }

    loop {
        tokio::select! {
            result = timeout(Duration::from_secs(config.event_timeout_secs), events.next()) => {
                match result {
                    Ok(Some(Ok(event))) => {
                        process_event(docker, config, notifiers, tracker, metrics, &mut dedup, event).await;
                    }
                    Ok(Some(Err(e))) => return Err(Box::new(e)),
                    Ok(None) => return Ok(()),
                    Err(_) => {
                        debug!("No events for {}s, pinging Docker...", config.event_timeout_secs);
                        docker.ping().await.map_err(|e| {
                            error!("Docker unresponsive: {}", e);
                            Box::new(e) as Box<dyn std::error::Error>
                        })?;
                    }
                }
            }
            _ = heartbeat.tick() => {
                info!("[HEARTBEAT] Watchdog is alive.");
                metrics.log_summary();
            }
            _ = cleanup_timer.tick() => {
                tracker.cleanup_stale(Duration::from_secs(STALE_CLEANUP_SECS));
                dedup.cleanup();
            }
        }
    }
}

async fn process_event(
    docker: &bollard::Docker,
    config: &AppConfig,
    notifiers: &[Box<dyn Notifier>],
    tracker: &mut RestartTracker,
    metrics: &mut CrashMetrics,
    dedup: &mut EventDedup,
    event: bollard::models::EventMessage,
) {
    if event.typ != Some(EventMessageTypeEnum::CONTAINER) {
        return;
    }

    let action = match event.action.as_deref() {
        Some(a) => a,
        None => return,
    };

    let actor = match event.actor {
        Some(ref a) => a,
        None => return,
    };

    let attrs = actor.attributes.as_ref();
    let full_id = actor.id.as_deref().unwrap_or("unknown");
    let name = attrs
        .and_then(|a| a.get("name").map(|s| s.as_str()))
        .unwrap_or("unknown");

    if config.ignore_containers.iter().any(|ignored| ignored == name) {
        return;
    }

    let overrides = LabelOverrides::from_attrs(attrs);
    if !overrides.enabled {
        return;
    }

    if action.starts_with("health_status") {
        handle_health_status(action, name);
        return;
    }

    if action == "start" {
        debug!("Container '{}' started.", name);
        return;
    }

    if !action.starts_with("die") {
        return;
    }

    if dedup.is_duplicate(full_id, action) {
        debug!("Suppressing duplicate 'die' event for container '{}'.", name);
        return;
    }

    // --- Container died — allocate only from here ---
    let short = utils::short_id(full_id);
    let exit_code = attrs
        .and_then(|a| a.get("exitCode"))
        .and_then(|s| s.parse::<i32>().ok());

    let container_event = ContainerEvent {
        container_id: short.to_string(),
        container_name: name.to_string(),
        action: action.to_string(),
        exit_code,
    };

    info!("[CONTAINER DIED] {}", container_event.summary());
    metrics.record_crash(name);

    if exit_code == Some(137) {
        warn!("Container '{}' was OOM-killed (exit code 137). Consider increasing memory limits.", name);
    }

    let logs = logger::fetch_logs(docker, full_id, config.log_tail_lines).await;
    let display_logs = utils::truncate_logs(&logs, 100);

    for notifier in notifiers {
        if let Err(e) = notifier.notify(&container_event, &display_logs).await {
            error!("Notifier '{}' failed: {}", notifier.name(), e);
        }
    }

    if config.auto_restart && container_event.is_failure() {
        match tracker.try_restart(docker, full_id, name, overrides.max_restarts).await {
            Ok(true) => info!("Container '{}' restart initiated.", name),
            Ok(false) => warn!("Container '{}' hit restart limit.", name),
            Err(e) => error!("Restart error for '{}': {}", name, e),
        }
    }
}

fn handle_health_status(action: &str, name: &str) {
    if action.contains("unhealthy") {
        warn!("[HEALTH] Container '{}' is unhealthy. Health check is failing.", name);
    } else if action.contains("healthy") {
        info!("[HEALTH] Container '{}' is healthy.", name);
    } else {
        debug!("[HEALTH] Container '{}' health_status: {}", name, action);
    }
}

async fn auto_detect_compose(config: &AppConfig, docker: &bollard::Docker) -> AppConfig {
    if config.compose_project.is_some() {
        return config.clone();
    }

    let hostname = match std::env::var("HOSTNAME") {
        Ok(h) if !h.is_empty() => h,
        _ => return config.clone(),
    };

    let project = docker
        .inspect_container(&hostname, None)
        .await
        .ok()
        .and_then(|info| info.config)
        .and_then(|c| c.labels)
        .and_then(|labels| labels.get("com.docker.compose.project").cloned());

    match project {
        Some(p) => {
            info!("Auto-detected compose project: '{}'", p);
            let mut cfg = config.clone();
            cfg.compose_project = Some(p);
            cfg
        }
        None => config.clone(),
    }
}
