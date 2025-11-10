use nym_network_defaults::NymNetworkDetails;
use nym_network_defaults::setup_env;
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, GroupSigningClient};
use nym_validator_client::{Config, DirectSigningHttpRpcNyxdClient, QueryHttpRpcValidatorClient};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum QueryMsg {
    GetCounter {},
}

#[derive(Serialize, Deserialize)]
pub enum ExecuteMsg {
    IncrementCounter {},
    DecrementCounter {},
    SetCounter { to: u32 },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    /*
    theres map
    id1 => wasm code 1
    id2 => wasm code 2
    id3 => wasm code 3

    //
    // n1foo => id3
    // n1bar => id2
    //


     */

    let contract_address = "n1vwttwvy8e5nqshc35rsn2u88ewfecjwdkmqdwpuqdsfmweans3xs2rjgjx";

    let mnemonic = "cost truly december route shoulder ostrich upon test test deliver moment tent general clutch manual language antenna curious gate library remember cost kidney proud";

    let network = "/Users/jedrzej/workspace/nym/envs/canary.env";
    setup_env(Some(network));

    let nym_network_details = NymNetworkDetails::new_from_env();

    let signing_client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
        nym_validator_client::nyxd::Config::try_from_nym_network_details(&nym_network_details)?,
        "https://rpc.canary-validator.performance.nymte.ch",
        mnemonic.parse()?,
    )?;

    let result = signing_client
        .execute(
            &contract_address.parse().unwrap(),
            &ExecuteMsg::SetCounter { to: 200 },
            None,
            "executing example contract",
            vec![],
        )
        .await?;
    println!("transaction got executed at tx {}", result.transaction_hash);
    println!("with events: {:?}", result.events);

    // queries:
    let client = QueryHttpRpcValidatorClient::new_query(Config::try_from_nym_network_details(
        &nym_network_details,
    )?)?;

    let response: u32 = client
        .nyxd
        .query_contract_smart(&contract_address.parse().unwrap(), &QueryMsg::GetCounter {})
        .await?;

    println!("current counter is at {response}");
    println!("Hello, world!");
    Ok(())
}
