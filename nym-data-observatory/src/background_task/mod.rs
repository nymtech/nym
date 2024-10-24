use core::str;
use serde::Deserialize;
use tokio::process::Command;
use tokio::task::JoinHandle;
use tokio::time::Duration;

use crate::db::{self, DbPool};

const REFRESH_DELAY: Duration = Duration::from_secs(15);
const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60 * 2);

pub(crate) async fn spawn_in_background(db_pool: DbPool) -> JoinHandle<()> {
    loop {
        tracing::info!("Running in a loop 🏃");

        if let Err(e) = some_network_action(&db_pool).await {
            tracing::error!(
                "❌ Run failed: {e}, retrying in {}s...",
                FAILURE_RETRY_DELAY.as_secs()
            );
            tokio::time::sleep(FAILURE_RETRY_DELAY).await;
        } else {
            tracing::info!(
                "✅ Run successful, sleeping for {}s...",
                REFRESH_DELAY.as_secs()
            );
            tokio::time::sleep(REFRESH_DELAY).await;
        }
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct Response {
    #[serde(rename(deserialize = "id"))]
    pub(crate) joke_id: String,
    pub(crate) joke: String,
    #[serde(rename(deserialize = "status"))]
    pub(crate) _status: u16,
}

async fn some_network_action(pool: &DbPool) -> anyhow::Result<()> {
    // for demonstration purposes only. You should use reqwest if you need it
    let output = Command::new("curl")
        .arg("-H")
        .arg("Accept: application/json")
        .arg("https://icanhazdadjoke.com/")
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!("Curl command failed with status: {}", output.status);
    }

    let response_str = str::from_utf8(&output.stdout)?;
    let joke_response: Response = serde_json::from_str(response_str)?;

    tracing::info!("{:?}", joke_response.joke);
    db::queries::insert_joke(pool, joke_response.into()).await?;

    Ok(())
}
