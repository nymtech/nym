use std::str::FromStr;

use crate::db::models::{GatewayRecord, MixnodeRecord};
use crate::db::{queries, DbPool, Storage};
use crate::error::NodeStatusApiResult;
use crate::read_env_var;
use anyhow::anyhow;
use cosmwasm_std::Decimal;
use nym_explorer_client::{ExplorerClient, PrettyDetailedGatewayBond};
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::models::{DescribedGateway, DescribedMixNode, MixNodeBondAnnotated};
use nym_validator_client::nym_nodes::SkimmedNode;
use nym_validator_client::nyxd::contract_traits::PagedMixnetQueryClient;
use nym_validator_client::nyxd::{AccountId, NyxdClient};
use nym_validator_client::NymApiClient;
use tokio::task::JoinHandle;
use tokio::time::Duration;

const REFRESH_DELAY: Duration = Duration::from_secs(60 * 5);
const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60);
// TODO dz is this info public?
// static DELEGATION_PROGRAM_WALLET: &str = "";

pub(crate) fn spawn_in_background(storage: Storage) -> JoinHandle<()> {
    tokio::spawn(async move {
        let db_pool = storage.pool().await;
        let network_defaults = nym_network_defaults::NymNetworkDetails::new_from_env();

        loop {
            tracing::info!("Refreshing node info...");

            if let Err(e) = run(db_pool, &network_defaults).await {
                tracing::error!(
                    "Monitor run failed: {e}, retrying in {}s...",
                    FAILURE_RETRY_DELAY.as_secs()
                );
                tokio::time::sleep(FAILURE_RETRY_DELAY).await;
            } else {
                tokio::time::sleep(REFRESH_DELAY).await;
            }
        }
    })
}

async fn run(db_pool: &DbPool, network_details: &NymNetworkDetails) -> NodeStatusApiResult<()> {
    let default_api_url = network_details
        .endpoints
        .first()
        .expect("rust sdk mainnet default incorrectly configured")
        .api_url
        .clone()
        .expect("rust sdk mainnet default missing api_url")
        .parse()
        .expect("rust sdk mainnet default api_url not parseable");
    let default_explorer_url = network_details.explorer_api.clone().map(|url| {
        url.parse()
            .expect("rust sdk mainnet default explorer url not parseable")
    });

    let default_explorer_url =
        default_explorer_url.expect("explorer url missing in network config");
    let explorer_client = ExplorerClient::new(default_explorer_url)?;
    let explorer_gateways = explorer_client.get_gateways().await?;
    tracing::debug!("explorer_gateways:\n{}", explorer_gateways.len());

    let api_client = NymApiClient::new(default_api_url);
    let gateways = api_client.get_cached_described_gateways().await?;
    tracing::debug!("Gateways:\n{}", gateways.len());
    tracing::debug!("example gateway:\n{:#?}", gateways.first());
    let skimmed_gateways = api_client.get_basic_gateways(None).await?;

    let mixnodes = api_client.get_cached_mixnodes().await?;
    tracing::debug!("Mixnodes:\n{}", mixnodes.len());
    // tracing::debug!("example mixnode:\n{:#?}", mixnodes.first());

    let mixnodes_described = api_client.nym_api.get_mixnodes_described().await?;
    tracing::debug!("Mixnodes described:\n{}", mixnodes_described.len());
    // tracing::debug!("Mixnodes described example:\n{:#?}", mixnodes_described.first());
    let gateways_blacklisted = api_client.nym_api.get_gateways_blacklisted().await?;
    tracing::debug!("gateways_blacklisted:\n{}", gateways_blacklisted.len());
    let mixnodes_blacklisted = api_client.nym_api.get_mixnodes_blacklisted().await?;
    tracing::debug!("mixnodes_blacklisted:\n{}", mixnodes_blacklisted.len());
    // TODO left over here

    // Cached mixnodes don't include blacklisted nodes
    // We need that to calculate the total locked tokens later
    let mixnodes = api_client
        .nym_api
        .get_mixnodes_detailed_unfiltered()
        .await?;
    let _mixnodes_described = api_client.nym_api.get_mixnodes_described().await?;
    let mixnodes_active = api_client.nym_api.get_active_mixnodes().await?;
    let delegation_program_members = get_delegation_program_details(network_details).await?;

    // keep stats for later
    let _count_bonded_mixnodes = mixnodes.len();
    let _count_bonded_gateways = gateways.len();
    let _count_explorer_gateways = explorer_gateways.len();
    let _count_bonded_mixnodes_active = mixnodes_active.len();

    let _conn = db_pool.acquire().await?;

    let gateway_records = prepare_gateways(
        gateways,
        &gateways_blacklisted,
        explorer_gateways,
        skimmed_gateways,
    )?;
    queries::insert_gateways(db_pool, gateway_records)
        .await
        .map(|_| {
            tracing::debug!("Gateway info written to DB!");
        })?;

    // TODO dz: isn't this the same as `count_gateways_blacklisted` ?
    let blacklisted_count = gateways_blacklisted.len();
    if blacklisted_count > 0 {
        queries::write_blacklisted_gateways_to_db(db_pool, gateways_blacklisted)
            .await
            .map(|_| {
                tracing::debug!(
                    "Gateway blacklist info written to DB! {} blacklisted by Nym API",
                    blacklisted_count
                )
            })?;
    }

    let mixnode_records =
        prepare_mixnodes(mixnodes, mixnodes_described, delegation_program_members).await?;
    queries::insert_mixnodes(db_pool, mixnode_records)
        .await
        .map(|_| {
            tracing::debug!("Mixnode info written to DB!");
        })?;

    Ok(())
}

fn prepare_gateways(
    gateways: Vec<DescribedGateway>,
    gateways_blacklisted: &Vec<String>,
    explorer_gateways: Vec<PrettyDetailedGatewayBond>,
    skimmed_gateways: Vec<SkimmedNode>,
) -> anyhow::Result<Vec<GatewayRecord>> {
    let mut gateway_records = Vec::new();

    for gateway in gateways.clone() {
        let gateway_identity_key = gateway.bond.identity();
        let bonded = true;
        let last_updated_utc = chrono::offset::Utc::now().timestamp();
        let blacklisted = gateways_blacklisted
            .iter()
            .any(|g| g == gateway_identity_key);

        // TODO dz removed, because it's calculated outside this fn call from Vec
        // if blacklisted {
        //     count_gateways_blacklisted += 1;
        // }

        let self_described = gateway
            .self_described
            .and_then(|v| serde_json::to_string(&v).ok());

        let explorer_pretty_bond = explorer_gateways
            .iter()
            .find(|g| g.gateway.identity_key.eq(gateway_identity_key));
        let explorer_pretty_bond = explorer_pretty_bond.and_then(|g| serde_json::to_string(g).ok());

        let gateway_performance = skimmed_gateways
            .iter()
            .find(|g| g.ed25519_identity_pubkey.eq(gateway_identity_key))
            .map(|g| g.performance)
            .unwrap_or_default()
            .round_to_integer();

        gateway_records.push((
            gateway_identity_key.clone(),
            bonded,
            blacklisted,
            self_described,
            explorer_pretty_bond,
            last_updated_utc,
            gateway_performance,
        ));
    }

    Ok(gateway_records)
}

async fn prepare_mixnodes(
    mixnodes: Vec<MixNodeBondAnnotated>,
    mixnodes_described: Vec<DescribedMixNode>,
    delegation_program_members: Vec<u32>,
) -> anyhow::Result<Vec<MixnodeRecord>> {
    let mut mixnode_records = Vec::new();

    for mixnode in mixnodes.clone() {
        let mix_id = mixnode.mix_id();
        let identity_key = mixnode.identity_key();
        let bonded = true;
        let total_stake = decimal_to_i64(mixnode.mixnode_details.total_stake());
        let blacklisted = mixnode.blacklisted;
        let node_info = mixnode.mix_node();
        let host = node_info.host.clone();
        let http_port = node_info.http_api_port;
        // Contains all the information including what's above
        let full_details = serde_json::to_string(&mixnode)?;

        let mixnode_described = mixnodes_described.iter().find(|m| m.bond.mix_id == mix_id);
        let self_described = mixnode_described.and_then(|v| serde_json::to_string(v).ok());
        let is_dp_delegatee = delegation_program_members.contains(&mix_id);

        // TODO dz removed, because it's calculated outside this fn call from Vec
        // if blacklisted {
        //     count_mixnodes_blacklisted += 1;
        // }

        let last_updated_utc = chrono::offset::Utc::now().timestamp();

        mixnode_records.push((
            mix_id,
            identity_key.to_string(),
            bonded,
            total_stake,
            host,
            http_port,
            blacklisted,
            full_details,
            self_described,
            last_updated_utc,
            is_dp_delegatee,
        ));
    }

    Ok(mixnode_records)
}

async fn get_delegation_program_details(
    network_details: &NymNetworkDetails,
) -> anyhow::Result<Vec<u32>> {
    let config = nym_validator_client::nyxd::Config::try_from_nym_network_details(network_details)?;

    // TODO dz should this be configurable?
    let client = NyxdClient::connect(config, "https://rpc.nymtech.net")
        .map_err(|err| anyhow::anyhow!("Couldn't connect: {}", err))?;

    let delegation_program_wallet = read_env_var("DELEGATION_PROGRAM_WALLET")?;
    let account_id = AccountId::from_str(&delegation_program_wallet)
        .map_err(|e| anyhow!("Invalid bech32 address: {}", e))?;

    let delegations = client.get_all_delegator_delegations(&account_id).await?;

    let mix_ids: Vec<u32> = delegations
        .iter()
        .map(|delegation| delegation.mix_id)
        .collect();

    Ok(mix_ids)
}

fn decimal_to_i64(decimal: Decimal) -> i64 {
    // Convert the underlying Uint128 to a u128
    let atomics = decimal.atomics().u128();
    let precision = 1_000_000_000_000_000_000u128;

    // Get the fractional part
    let fractional = atomics % precision;

    // Get the integer part
    let integer = atomics / precision;

    // Combine them into a float
    let float_value = integer as f64 + (fractional as f64 / 1_000_000_000_000_000_000_f64);

    // Limit to 6 decimal places
    let rounded_value = (float_value * 1_000_000.0).round() / 1_000_000.0;

    rounded_value as i64
}
