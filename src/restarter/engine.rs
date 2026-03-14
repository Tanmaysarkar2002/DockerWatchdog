use std::collections::HashMap;
use bollard::Docker;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// Tracks restart attempts per container and handles auto-restart logic.
pub struct RestartTracker {
    /// Maps container_id -> number of restart attempts so far.
    attempts: HashMap<String, u32>,

    /// Maximum restarts allowed per container.
    max_restarts: u32,

    /// Delay in seconds before attempting a restart.
    delay_secs: u64,
}

impl RestartTracker {
    /// Creates a new RestartTracker with the given limits.
    pub fn new(max_restarts: u32, delay_secs: u64) -> Self {
        Self {
            attempts: HashMap::new(),
            max_restarts,
            delay_secs,
        }
    }

    /// Checks if a restart is allowed and increments the attempt counter.
    /// Returns `true` if a restart should proceed, `false` if the limit is reached.
    /// This is separated from `try_restart` so it can be unit-tested without Docker.
    pub fn should_restart(&mut self, container_id: &str, container_name: &str) -> bool {
        let count = self.attempts.entry(container_id.to_string()).or_insert(0);

        if *count >= self.max_restarts {
            warn!(
                "Container '{}' ({}) has reached max restart limit ({}/{}). Giving up.",
                container_name, container_id, count, self.max_restarts
            );
            return false;
        }

        *count += 1;
        info!(
            "Restarting container '{}' ({}) in {} seconds... (attempt {}/{})",
            container_name, container_id, self.delay_secs, count, self.max_restarts
        );
        true
    }

    /// Attempts to restart a container. Returns Ok(true) if restarted,
    /// Ok(false) if the max restart limit was reached, or Err on failure.
    pub async fn try_restart(
        &mut self,
        docker: &Docker,
        container_id: &str,
        container_name: &str,
    ) -> Result<bool, String> {
        if !self.should_restart(container_id, container_name) {
            return Ok(false);
        }

        // Wait before restarting
        sleep(Duration::from_secs(self.delay_secs)).await;

        // Attempt the restart via Docker API
        match docker
            .restart_container(container_id, None)
            .await
        {
            Ok(_) => {
                info!(
                    "Container '{}' ({}) restarted successfully.",
                    container_name, container_id
                );
                Ok(true)
            }
            Err(e) => {
                error!(
                    "Failed to restart container '{}' ({}): {}",
                    container_name, container_id, e
                );
                Err(format!("Restart failed: {}", e))
            }
        }
    }

    /// Resets the restart counter for a container (e.g., when it starts cleanly).
    pub fn reset(&mut self, container_id: &str) {
        self.attempts.remove(container_id);
    }

    /// Returns the current restart count for a container.
    pub fn restart_count(&self, container_id: &str) -> u32 {
        *self.attempts.get(container_id).unwrap_or(&0)
    }
}
