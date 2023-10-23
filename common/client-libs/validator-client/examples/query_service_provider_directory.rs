use std::str::FromStr;

use cosmrs::AccountId;
use nym_network_defaults::{setup_env, NymNetworkDetails};
use nym_service_provider_directory_common::NymAddress;
use nym_validator_client::nyxd::contract_traits::{
    PagedSpDirectoryQueryClient, SpDirectoryQueryClient,
};

#[tokio::main]
async fn main() {
    setup_env(Some("../../../envs/qa.env"));
    let network_details = NymNetworkDetails::new_from_env();
    let config =
        nym_validator_client::Config::try_from_nym_network_details(&network_details).unwrap();
    let client = nym_validator_client::Client::new_query(config).unwrap();

    let config = client.nyxd.get_service_config().await.unwrap();
    println!("config: {config:?}");

    let services_paged = client.nyxd.get_services_paged(None, None).await.unwrap();
    println!("services (paged): {services_paged:#?}");

    let services = client.nyxd.get_all_services().await.unwrap();
    println!("services: {services:#?}");

    let announcer = AccountId::from_str("n1hmf957kc7arcd39rl7xq8l0a4zyg7kxnv7su87").unwrap();
    let services_by_announcer = client
        .nyxd
        .get_services_by_announcer(announcer)
        .await
        .unwrap();
    println!("services (by announcer): {services_by_announcer:#?}");

    let nym_address = NymAddress::new("foo.bar@gateway");
    let services_by_nym_address = client
        .nyxd
        .get_services_by_nym_address(nym_address)
        .await
        .unwrap();
    assert_eq!(services_by_announcer, services_by_nym_address);

    let service_info = client.nyxd.get_service_info(1).await;
    println!("service info: {service_info:#?}");
}
