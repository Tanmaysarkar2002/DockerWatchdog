use std::collections::HashMap;
use bollard::Docker;
use tokio::time::{Instant, sleep, Duration};
use tracing::{debug, error, info, warn};

struct ContainerState {
    attempts: u32,
    last_restart: Option<Instant>,
}

pub struct RestartTracker {
    containers: HashMap<String, ContainerState>,
    max_restarts: u32,
    base_delay_secs: u64,
    backoff: bool,
    cooldown_secs: u64,
    dry_run: bool,
}

impl RestartTracker {
    pub fn new(
        max_restarts: u32,
        base_delay_secs: u64,
        backoff: bool,
        cooldown_secs: u64,
        dry_run: bool,
    ) -> Self {
        Self {
            containers: HashMap::new(),
            max_restarts,
            base_delay_secs,
            backoff,
            cooldown_secs,
            dry_run,
        }
    }

    /// Resets counter if container ran longer than cooldown since last restart.
    fn apply_cooldown(&mut self, container_id: &str) {
        if self.cooldown_secs == 0 {
            return;
        }
        if let Some(state) = self.containers.get(container_id) {
            if let Some(last) = state.last_restart {
                if last.elapsed() >= Duration::from_secs(self.cooldown_secs) {
                    debug!(
                        "Container {} ran for {}s (cooldown={}s), resetting restart counter.",
                        container_id, last.elapsed().as_secs(), self.cooldown_secs
                    );
                    self.containers.remove(container_id);
                }
            }
        }
    }

    fn compute_delay(&self, attempt: u32) -> u64 {
        if self.backoff && attempt > 1 {
            // Exponential: base * 2^(attempt-1), capped at 5 minutes
            let delay = self.base_delay_secs.saturating_mul(1u64 << (attempt - 1).min(6));
            delay.min(300)
        } else {
            self.base_delay_secs
        }
    }

    pub fn should_restart(
        &mut self,
        container_id: &str,
        container_name: &str,
        max_override: Option<u32>,
    ) -> bool {
        self.apply_cooldown(container_id);
        let effective_max = max_override.unwrap_or(self.max_restarts);

        // Fast path: check without allocating if already at limit
        if let Some(state) = self.containers.get(container_id) {
            if state.attempts >= effective_max {
                warn!(
                    "Container '{}' ({}) has reached max restart limit ({}/{}). Giving up.",
                    container_name, container_id, state.attempts, effective_max
                );
                return false;
            }
        }

        let state = self.containers.entry(container_id.to_string()).or_insert(ContainerState {
            attempts: 0,
            last_restart: None,
        });

        if state.attempts >= effective_max {
            warn!(
                "Container '{}' ({}) has reached max restart limit ({}/{}). Giving up.",
                container_name, container_id, state.attempts, effective_max
            );
            return false;
        }

        state.attempts += 1;
        state.last_restart = Some(Instant::now());
        let attempts = state.attempts;

        let delay = self.compute_delay(attempts);
        info!(
            "Restarting container '{}' ({}) in {}s... (attempt {}/{})",
            container_name, container_id, delay, attempts, effective_max
        );
        true
    }

    pub async fn try_restart(
        &mut self,
        docker: &Docker,
        container_id: &str,
        container_name: &str,
        max_override: Option<u32>,
    ) -> Result<bool, String> {
        if !self.should_restart(container_id, container_name, max_override) {
            return Ok(false);
        }

        let attempt = self.containers.get(container_id).map_or(0, |s| s.attempts);
        let delay = self.compute_delay(attempt);

        if self.dry_run {
            warn!(
                "[DRY-RUN] Would restart container '{}' ({}) after {}s delay (attempt {}).",
                container_name, container_id, delay, attempt
            );
            return Ok(true);
        }

        sleep(Duration::from_secs(delay)).await;

        match docker.restart_container(container_id, None).await {
            Ok(_) => {
                info!("Container '{}' ({}) restarted successfully.", container_name, container_id);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to restart container '{}' ({}): {}", container_name, container_id, e);
                Err(format!("Restart failed: {}", e))
            }
        }
    }

    pub fn reset(&mut self, container_id: &str) {
        self.containers.remove(container_id);
    }

    pub fn restart_count(&self, container_id: &str) -> u32 {
        self.containers.get(container_id).map_or(0, |s| s.attempts)
    }

    pub fn cleanup_stale(&mut self, max_age: Duration) {
        self.containers.retain(|id, state| {
            let keep = state.last_restart.map_or(true, |t| t.elapsed() < max_age);
            if !keep {
                debug!("Cleaning up stale restart tracking for container {}.", id);
            }
            keep
        });
    }
}
