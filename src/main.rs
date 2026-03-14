use docker_watchdog::config::AppConfig;
use docker_watchdog::logger;
use docker_watchdog::monitor;
use docker_watchdog::notifier::ConsoleNotifier;

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load configuration
    let config = AppConfig::from_env();

    // 2. Initialize logging
    logger::init_tracing(&config.log_level);

    info!("docker-watchdog v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration: {:?}", config);

    // 3. Set up notifiers
    let notifiers: Vec<Box<dyn docker_watchdog::notifier::Notifier>> = vec![
        Box::new(ConsoleNotifier::new()),
        // Future: Box::new(SlackNotifier::new(...)),
        // Future: Box::new(KafkaNotifier::new(...)),
    ];

    // 4. Start the monitoring loop (handles Docker connections internally)
    monitor::run(&config, &notifiers).await?;

    Ok(())
}
