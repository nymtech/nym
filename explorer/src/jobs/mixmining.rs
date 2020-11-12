use crate::utils::file;
use reqwest::Error;

const RELATIVE_PATH: &str = "api/mixmining/fullreport";

pub async fn renew_periodically(validator_base_url: &str) -> Result<(), Error> {
    let url = format!("{}/{}", validator_base_url, RELATIVE_PATH);

    let topology_json = reqwest::get(&url).await?.text().await?;
    file::save(topology_json, "public/downloads/mixmining.json");
    Ok(())
}
