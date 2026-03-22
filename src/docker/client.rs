use bollard::Docker;
use tracing::info;

pub async fn connect() -> Result<Docker, bollard::errors::Error> {
    let docker = Docker::connect_with_local_defaults()?;
    docker.ping().await?;
    info!("Connected to Docker daemon.");
    Ok(docker)
}
