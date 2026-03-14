/// Represents a parsed Docker container event that we care about.
#[derive(Debug, Clone)]
pub struct ContainerEvent {
    /// The container ID (short hash).
    pub container_id: String,

    /// The container name (human-readable).
    pub container_name: String,

    /// The action that occurred (e.g., "die", "stop", "start").
    pub action: String,

    /// Exit code if the container died (e.g., "0", "1", "137").
    pub exit_code: Option<String>,
}

impl ContainerEvent {
    /// Returns true if this event represents a container crash or failure.
    pub fn is_failure(&self) -> bool {
        if let Some(ref code) = self.exit_code {
            code != "0"
        } else {
            // If no exit code, we treat "die" as a potential failure.
            self.action == "die"
        }
    }

    /// Returns a formatted summary string suitable for logging.
    pub fn summary(&self) -> String {
        let code_str = self
            .exit_code
            .as_deref()
            .unwrap_or("unknown");
        format!(
            "Container '{}' ({}) action='{}' exit_code={}",
            self.container_name, self.container_id, self.action, code_str
        )
    }
}
