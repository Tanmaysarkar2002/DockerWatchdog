use std::future::Future;
use std::pin::Pin;
use crate::docker::ContainerEvent;

/// Trait that all notifiers must implement.
/// Uses a manually boxed future for object safety without the async-trait crate.
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;
    fn notify<'a>(&'a self, event: &'a ContainerEvent, logs: &'a str)
        -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}
