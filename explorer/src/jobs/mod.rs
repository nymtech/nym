use log::*;
use tokio::time::{self, Duration};

mod mixmining;
mod topology;

pub async fn start(validator_base_url: &str) {
    let mut timer = time::interval(Duration::from_secs(10));
    loop {
        timer.tick().await;

        if let Err(err) = topology::renew_periodically(validator_base_url).await {
            warn!("Error refreshing topology: {}", err)
        };

        if let Err(err) = mixmining::renew_periodically(validator_base_url).await {
            warn!("Error refreshing mixmining report: {}", err)
        };
    }
}
