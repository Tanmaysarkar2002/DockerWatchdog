use docker_watchdog::config::AppConfig;
use docker_watchdog::logger;
use docker_watchdog::monitor;
use docker_watchdog::notifier::ConsoleNotifier;

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env();
    logger::init_tracing(&config.log_level);

    info!("docker-watchdog v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration: {:?}", config);

    let notifiers: Vec<Box<dyn docker_watchdog::notifier::Notifier>> = vec![
        Box::new(ConsoleNotifier::new()),
    ];

    tokio::select! {
        result = monitor::run(&config, &notifiers) => result?,
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal. Exiting gracefully.");
        }
    }

    Ok(())
}
