use anyhow::Result;
use habitat_controller::manager::Manager;
use tracing::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let client = kube::Client::try_default().await?;
    let (_manager, controller) = Manager::new(client).await;

    tokio::select! {
        _ = controller => warn!("controller exited"),
    }

    Ok(())
}
