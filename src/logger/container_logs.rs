use bollard::query_parameters::LogsOptions;
use bollard::Docker;
use futures_util::StreamExt;
use tracing::{error, warn};

/// Fetches the last N lines of logs from a container and returns them as a String.
pub async fn fetch_logs(docker: &Docker, container_id: &str, tail_lines: u64) -> String {
    let opts = Some(LogsOptions {
        stdout: true,
        stderr: true,
        tail: tail_lines.to_string(),
        ..Default::default()
    });

    let mut stream = docker.logs(container_id, opts);
    let mut output = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(log) => output.push_str(&log.to_string()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("dead or marked for removal") || msg.contains("409") {
                    warn!("Container {} was removed (--rm flag?). Logs unavailable.", container_id);
                } else {
                    error!("Error fetching logs for {}: {}", container_id, e);
                }
                break;
            }
        }
    }

    output
}
