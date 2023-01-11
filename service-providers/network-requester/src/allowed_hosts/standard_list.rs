/// Fetch the standard allowed list from nymtech.net
pub(crate) async fn fetch() -> Vec<String> {
    log::info!("Refreshing standard allowed hosts");
    get_standard_allowed_list()
        .await
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

async fn get_standard_allowed_list() -> String {
    reqwest::get("https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt")
        .await
        .expect("failed to get allowed hosts")
        .text()
        .await
        .expect("failed to get allowed hosts text")
}
