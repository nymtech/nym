use nym_http_api_client::{ApiClient, Client};
use nym_validator_client::nym_api::NymApiClientExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = "https://validator.nymtech.net/api/";

    let client = Client::new(url.parse()?, None);

    let res = client.get_rewarded_set().await;
    println!("{:?}", res);
    println!("Hello, world!");

    Ok(())
}
