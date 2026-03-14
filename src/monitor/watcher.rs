use bollard::models::EventMessageTypeEnum;
use bollard::query_parameters::EventsOptions;
use futures_util::StreamExt;
use tokio::time::{Duration, interval, timeout};
use tracing::{debug, error, info, warn};

use crate::config::AppConfig;
use crate::docker::{self, ContainerEvent};
use crate::logger;
use crate::notifier::Notifier;
use crate::restarter::RestartTracker;
use crate::utils;

const HEARTBEAT_SECS: u64 = 300;
const EVENT_TIMEOUT_SECS: u64 = 600;
const MAX_RECONNECT_DELAY_SECS: u64 = 60;

/// Runs the watchdog with auto-reconnection.
pub async fn run(
    config: &AppConfig,
    notifiers: &[Box<dyn Notifier>],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = RestartTracker::new(config.max_restarts, config.restart_delay_secs);
    let mut attempts: u32 = 0;

    loop {
        match docker::connect().await {
            Ok(docker) => {
                if attempts > 0 {
                    info!("Reconnected to Docker daemon after {} attempts.", attempts);
                }
                attempts = 0;

                let effective_config = auto_detect_compose(config, &docker).await;

                if let Err(e) =
                    event_loop(&docker, &effective_config, notifiers, &mut tracker).await
                {
                    error!("Event loop error: {}. Reconnecting...", e);
                } else {
                    warn!("Docker event stream ended. Reconnecting...");
                }
            }
            Err(e) => {
                attempts += 1;
                let delay = std::cmp::min(2u64.saturating_pow(attempts), MAX_RECONNECT_DELAY_SECS);
                error!("Cannot reach Docker daemon: {} (retry in {}s)", e, delay);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
        }
        attempts += 1;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Inner event loop — processes Docker events until the stream ends or errors.
async fn event_loop(
    docker: &bollard::Docker,
    config: &AppConfig,
    notifiers: &[Box<dyn Notifier>],
    tracker: &mut RestartTracker,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut events = docker.events(Some(EventsOptions::default()));
    let mut heartbeat = interval(Duration::from_secs(HEARTBEAT_SECS));
    heartbeat.tick().await;

    info!("Watching for Docker container events...");
    if config.auto_restart {
        info!(
            "Auto-restart enabled (max {} attempts, {}s delay)",
            config.max_restarts, config.restart_delay_secs
        );
    }
    if let Some(ref project) = config.compose_project {
        info!("Filtering to compose project: '{}'", project);
    }

    loop {
        tokio::select! {
            result = timeout(Duration::from_secs(EVENT_TIMEOUT_SECS), events.next()) => {
                match result {
                    Ok(Some(Ok(event))) => process_event(docker, config, notifiers, tracker, event).await,
                    Ok(Some(Err(e))) => return Err(Box::new(e)),
                    Ok(None) => return Ok(()),
                    Err(_) => {
                        debug!("No events for 10 minutes, pinging Docker...");
                        docker.ping().await.map_err(|e| {
                            error!("Docker unresponsive: {}", e);
                            Box::new(e) as Box<dyn std::error::Error>
                        })?;
                    }
                }
            }
            _ = heartbeat.tick() => {
                info!("[HEARTBEAT] Watchdog is alive.");
            }
        }
    }
}

/// Processes a single Docker event. Avoids allocations until we know
/// the event is actionable (a "die" event in our compose project).
async fn process_event(
    docker: &bollard::Docker,
    config: &AppConfig,
    notifiers: &[Box<dyn Notifier>],
    tracker: &mut RestartTracker,
    event: bollard::models::EventMessage,
) {
    if event.typ != Some(EventMessageTypeEnum::CONTAINER) {
        return;
    }

    // Borrow action without cloning — only clone later if needed
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

    // Filter by compose project early — before any allocations
    if let Some(ref project) = config.compose_project {
        let container_project = attrs
            .and_then(|a| a.get("com.docker.compose.project").map(|s| s.as_str()))
            .unwrap_or("");
        if container_project != project.as_str() {
            return;
        }
    }

    if action == "start" {
        debug!("Container '{}' started.", name);
        return;
    }

    if !action.starts_with("die") {
        debug!("Container '{}' event: {}", name, action);
        return;
    }

    // --- Container died — NOW we allocate ---
    let short = utils::short_id(full_id);
    let exit_code = attrs.and_then(|a| a.get("exitCode").cloned());

    let container_event = ContainerEvent {
        container_id: short.to_string(),
        container_name: name.to_string(),
        action: action.to_string(),
        exit_code,
    };

    info!("[CONTAINER DIED] {}", container_event.summary());

    let logs = logger::fetch_logs(docker, full_id, config.log_tail_lines).await;
    let display_logs = utils::truncate_logs(&logs, 100);

    for notifier in notifiers {
        if let Err(e) = notifier.notify(&container_event, &display_logs).await {
            error!("Notifier '{}' failed: {}", notifier.name(), e);
        }
    }

    if config.auto_restart && container_event.is_failure() {
        match tracker.try_restart(docker, full_id, name).await {
            Ok(true) => info!("Container '{}' restart initiated.", name),
            Ok(false) => warn!("Container '{}' hit restart limit.", name),
            Err(e) => error!("Restart error for '{}': {}", name, e),
        }
    }
}

/// Auto-detects compose project from the watchdog's own container labels.
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
