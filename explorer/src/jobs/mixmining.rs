use reqwest::Error;

use crate::utils::file;

pub async fn renew_periodically() -> Result<(), Error> {
    let topology_json =
        reqwest::get("http://qa-validator.nymtech.net:8081/api/mixmining/fullreport")
            .await?
            .text()
            .await?;
    file::save(topology_json, "public/downloads/mixmining.json");
    Ok(())
}
