use std::future::Future;
use std::pin::Pin;
use tracing::warn;

use crate::docker::ContainerEvent;
use super::traits::Notifier;

pub struct ConsoleNotifier;

impl ConsoleNotifier {
    pub fn new() -> Self { Self }
}

impl Notifier for ConsoleNotifier {
    fn name(&self) -> &str { "console" }

    fn notify<'a>(&'a self, event: &'a ContainerEvent, logs: &'a str)
        -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
    {
        Box::pin(async move {
            warn!("ALERT: {}", event.summary());
            if event.is_failure() {
                warn!("Container '{}' exited with a non-zero exit code!", event.container_name);
            }
            if !logs.is_empty() {
                warn!("Last logs:\n{}", logs);
            }
            Ok(())
        })
    }
}
