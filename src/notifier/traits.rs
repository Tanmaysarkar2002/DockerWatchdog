use std::future::Future;
use std::pin::Pin;
use crate::docker::ContainerEvent;

/// Object-safe async notifier trait using manually boxed futures.
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;
    fn notify<'a>(&'a self, event: &'a ContainerEvent, logs: &'a str)
        -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}
