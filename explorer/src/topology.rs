use reqwest::Error;
use std::time::Duration;
use tokio::time;

use crate::utils::file;

pub async fn renew_periodically() -> Result<(), Error> {
    let mut interval_day = time::interval(Duration::from_secs(5));
    loop {
        interval_day.tick().await;
        let topology_json =
            reqwest::get("http://qa-validator.nymtech.net:8081/api/mixmining/topology")
                .await?
                .text()
                .await?;
        file::save(topology_json, "downloads/topology.json");
    }
}
