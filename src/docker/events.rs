#[derive(Debug, Clone)]
pub struct ContainerEvent {
    pub container_id: String,
    pub container_name: String,
    pub action: String,
    pub exit_code: Option<i32>,
}

impl ContainerEvent {
    pub fn is_failure(&self) -> bool {
        self.exit_code.map_or(self.action == "die", |code| code != 0)
    }

    pub fn summary(&self) -> String {
        let code_str = match self.exit_code {
            Some(code) => code.to_string(),
            None => "unknown".to_string(),
        };
        format!(
            "Container '{}' ({}) action='{}' exit_code={}",
            self.container_name, self.container_id, self.action, code_str
        )
    }
}
