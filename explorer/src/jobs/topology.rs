use crate::utils::file;
use reqwest::Error;

const RELATIVE_PATH: &str = "api/mixmining/topology";

pub async fn renew_periodically(validator_base_url: &str) -> Result<(), Error> {
    let url = format!("{}/{}", validator_base_url, RELATIVE_PATH);

    let topology_json = reqwest::get(&url).await?.text().await?;

    let save_path = std::env::current_exe()
        .expect("Failed to evaluate current exe path")
        .parent()
        .expect("the binary itself has no parent path?!")
        .join("public")
        .join("downloads")
        .join("topology.json");

    file::save(topology_json, save_path);
    Ok(())
}
