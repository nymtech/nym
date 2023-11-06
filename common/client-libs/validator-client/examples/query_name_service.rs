use std::str::FromStr;

use cosmrs::AccountId;
use nym_name_service_common::Address;
use nym_network_defaults::{setup_env, NymNetworkDetails};
use nym_validator_client::nyxd::contract_traits::{
    NameServiceQueryClient, PagedNameServiceQueryClient,
};

#[tokio::main]
async fn main() {
    setup_env(Some("../../../envs/qa.env"));
    let network_details = NymNetworkDetails::new_from_env();
    let config =
        nym_validator_client::Config::try_from_nym_network_details(&network_details).unwrap();
    let client = nym_validator_client::Client::new_query(config).unwrap();

    let config = client.nyxd.get_name_service_config().await.unwrap();
    println!("config: {config:?}");

    let names_paged = client.nyxd.get_names_paged(None, None).await.unwrap();
    println!("names (paged): {names_paged:#?}");

    let names = client.nyxd.get_all_names().await.unwrap();
    println!("names: {names:#?}");

    let owner = AccountId::from_str("n1hmf957kc7arcd39rl7xq8l0a4zyg7kxnv7su87").unwrap();
    let names_by_owner = client.nyxd.get_names_by_owner(owner).await.unwrap();
    println!("names (by owner): {names_by_owner:#?}");

    let nym_address = Address::new("client_id.client_key@gateway_id").unwrap();
    let names_by_address = client.nyxd.get_names_by_address(nym_address).await.unwrap();
    println!("names (by address): {names_by_address:#?}");

    let service_info = client.nyxd.get_name_entry(1).await;
    println!("service info: {service_info:#?}");
}
