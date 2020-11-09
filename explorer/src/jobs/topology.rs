use reqwest::Error;

use crate::utils::file;

pub async fn renew_periodically() -> Result<(), Error> {
    let topology_json = reqwest::get("http://qa-validator.nymtech.net:8081/api/mixmining/topology")
        .await?
        .text()
        .await?;
    file::save(topology_json, "public/downloads/topology.json");
    Ok(())
}
