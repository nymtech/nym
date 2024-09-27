use crate::db::models::{
    gateway, mixnode, GatewayRecord, MixnodeRecord, NetworkSummary, GATEWAYS_BLACKLISTED_COUNT,
    GATEWAYS_BONDED_COUNT, GATEWAYS_EXPLORER_COUNT, GATEWAYS_HISTORICAL_COUNT,
    MIXNODES_BLACKLISTED_COUNT, MIXNODES_BONDED_ACTIVE, MIXNODES_BONDED_COUNT,
    MIXNODES_BONDED_INACTIVE, MIXNODES_BONDED_RESERVE, MIXNODES_HISTORICAL_COUNT,
};
use crate::db::{queries, DbPool};
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
use std::collections::HashSet;
use std::str::FromStr;
use tokio::task::JoinHandle;
use tokio::time::Duration;

const REFRESH_DELAY: Duration = Duration::from_secs(60 * 5);
const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60);
static DELEGATION_PROGRAM_WALLET: &str = "n1rnxpdpx3kldygsklfft0gech7fhfcux4zst5lw";

// TODO dz: query many NYM APIs:
// multiple instances running directory cache, ask sachin
pub(crate) async fn spawn_in_background(db_pool: DbPool) -> JoinHandle<()> {
    tokio::spawn(async move {
        let network_defaults = nym_network_defaults::NymNetworkDetails::new_from_env();

        loop {
            tracing::info!("Refreshing node info...");

            if let Err(e) = run(&db_pool, &network_defaults).await {
                tracing::error!(
                    "Monitor run failed: {e}, retrying in {}s...",
                    FAILURE_RETRY_DELAY.as_secs()
                );
                tokio::time::sleep(FAILURE_RETRY_DELAY).await;
            } else {
                tracing::info!(
                    "Info successfully collected, sleeping for {}s...",
                    REFRESH_DELAY.as_secs()
                );
                tokio::time::sleep(REFRESH_DELAY).await;
            }
        }
    })
}

async fn run(pool: &DbPool, network_details: &NymNetworkDetails) -> anyhow::Result<()> {
    let default_api_url = network_details
        .endpoints
        .first()
        .expect("rust sdk mainnet default incorrectly configured")
        .api_url()
        .clone()
        .expect("rust sdk mainnet default missing api_url");
    let default_explorer_url = network_details.explorer_api.clone().map(|url| {
        url.parse()
            .expect("rust sdk mainnet default explorer url not parseable")
    });

    let default_explorer_url =
        default_explorer_url.expect("explorer url missing in network config");
    let explorer_client = ExplorerClient::new(default_explorer_url)?;
    let explorer_gateways = explorer_client.get_gateways().await?;

    let api_client = NymApiClient::new(default_api_url);
    let gateways = api_client.get_cached_described_gateways().await?;
    tracing::debug!("Fetched {} gateways", gateways.len());
    let skimmed_gateways = api_client.get_basic_gateways(None).await?;

    let mixnodes = api_client.get_cached_mixnodes().await?;
    tracing::debug!("Fetched {} mixnodes", mixnodes.len());

    // TODO dz can we calculate blacklisted GWs from their performance?
    // where do we get their performance?
    let gateways_blacklisted = api_client
        .nym_api
        .get_gateways_blacklisted()
        .await
        .map(|vec| vec.into_iter().collect::<HashSet<_>>())?;

    // Cached mixnodes don't include blacklisted nodes
    // We need that to calculate the total locked tokens later
    let mixnodes = api_client
        .nym_api
        .get_mixnodes_detailed_unfiltered()
        .await?;
    let mixnodes_described = api_client.nym_api.get_mixnodes_described().await?;
    let mixnodes_active = api_client.nym_api.get_active_mixnodes().await?;
    let delegation_program_members = get_delegation_program_details(network_details).await?;

    // keep stats for later
    let count_bonded_mixnodes = mixnodes.len();
    let count_bonded_gateways = gateways.len();
    let count_explorer_gateways = explorer_gateways.len();
    let count_bonded_mixnodes_active = mixnodes_active.len();

    let gateway_records = prepare_gateway_data(
        &gateways,
        &gateways_blacklisted,
        explorer_gateways,
        skimmed_gateways,
    )?;
    queries::insert_gateways(pool, gateway_records)
        .await
        .map(|_| {
            tracing::debug!("Gateway info written to DB!");
        })?;

    // instead of counting blacklisted GWs returned from API cache, count from the active set
    let count_gateways_blacklisted = gateways
        .iter()
        .filter(|gw| {
            let gw_identity = gw.bond.identity();
            gateways_blacklisted.contains(gw_identity)
        })
        .count();

    if count_gateways_blacklisted > 0 {
        queries::write_blacklisted_gateways_to_db(pool, gateways_blacklisted.iter())
            .await
            .map(|_| {
                tracing::debug!(
                    "Gateway blacklist info written to DB! {} blacklisted by Nym API",
                    count_gateways_blacklisted
                )
            })?;
    }

    let mixnode_records =
        prepare_mixnode_data(&mixnodes, mixnodes_described, delegation_program_members)?;
    queries::insert_mixnodes(pool, mixnode_records)
        .await
        .map(|_| {
            tracing::debug!("Mixnode info written to DB!");
        })?;

    let count_mixnodes_blacklisted = mixnodes.iter().filter(|elem| elem.blacklisted).count();

    let recently_unbonded_gateways = queries::ensure_gateways_still_bonded(pool, &gateways).await?;
    let recently_unbonded_mixnodes = queries::ensure_mixnodes_still_bonded(pool, &mixnodes).await?;

    let count_bonded_mixnodes_reserve = 0; // TODO: NymAPI doesn't report the reserve set size
    let count_bonded_mixnodes_inactive = count_bonded_mixnodes - count_bonded_mixnodes_active;

    let (all_historical_gateways, all_historical_mixnodes) = calculate_stats(pool).await?;

    //
    // write summary keys and values to table
    //

    let nodes_summary = vec![
        (MIXNODES_BONDED_COUNT, &count_bonded_mixnodes),
        (MIXNODES_BONDED_ACTIVE, &count_bonded_mixnodes_active),
        (MIXNODES_BONDED_INACTIVE, &count_bonded_mixnodes_inactive),
        (MIXNODES_BONDED_RESERVE, &count_bonded_mixnodes_reserve),
        (MIXNODES_BLACKLISTED_COUNT, &count_mixnodes_blacklisted),
        (GATEWAYS_BONDED_COUNT, &count_bonded_gateways),
        (GATEWAYS_EXPLORER_COUNT, &count_explorer_gateways),
        (MIXNODES_HISTORICAL_COUNT, &all_historical_mixnodes),
        (GATEWAYS_HISTORICAL_COUNT, &all_historical_gateways),
        (GATEWAYS_BLACKLISTED_COUNT, &count_gateways_blacklisted),
    ];

    // TODO dz do we need signed int in type definition? maybe because of API?
    let last_updated = chrono::offset::Utc::now();
    let last_updated_utc = last_updated.timestamp().to_string();
    let network_summary = NetworkSummary {
        mixnodes: mixnode::MixnodeSummary {
            bonded: mixnode::MixnodeSummaryBonded {
                count: count_bonded_mixnodes as i32,
                active: count_bonded_mixnodes_active as i32,
                inactive: count_bonded_mixnodes_inactive as i32,
                reserve: count_bonded_mixnodes_reserve as i32,
                last_updated_utc: last_updated_utc.to_owned(),
            },
            blacklisted: mixnode::MixnodeSummaryBlacklisted {
                count: count_mixnodes_blacklisted as i32,
                last_updated_utc: last_updated_utc.to_owned(),
            },
            historical: mixnode::MixnodeSummaryHistorical {
                count: all_historical_mixnodes as i32,
                last_updated_utc: last_updated_utc.to_owned(),
            },
        },
        gateways: gateway::GatewaySummary {
            bonded: gateway::GatewaySummaryBonded {
                count: count_bonded_gateways as i32,
                last_updated_utc: last_updated_utc.to_owned(),
            },
            blacklisted: gateway::GatewaySummaryBlacklisted {
                count: count_gateways_blacklisted as i32,
                last_updated_utc: last_updated_utc.to_owned(),
            },
            historical: gateway::GatewaySummaryHistorical {
                count: all_historical_gateways as i32,
                last_updated_utc: last_updated_utc.to_owned(),
            },
            explorer: gateway::GatewaySummaryExplorer {
                count: count_explorer_gateways as i32,
                last_updated_utc: last_updated_utc.to_owned(),
            },
        },
    };

    queries::insert_summaries(pool, &nodes_summary, &network_summary, last_updated).await?;

    let mut log_lines: Vec<String> = vec![];
    for (key, value) in nodes_summary.iter() {
        log_lines.push(format!("{} = {}", key, value));
    }
    log_lines.push(format!(
        "recently_unbonded_mixnodes = {}",
        recently_unbonded_mixnodes
    ));
    log_lines.push(format!(
        "recently_unbonded_gateways = {}",
        recently_unbonded_gateways
    ));

    tracing::info!("Directory summary: \n{}", log_lines.join("\n"));

    Ok(())
}

fn prepare_gateway_data(
    gateways: &[DescribedGateway],
    gateways_blacklisted: &HashSet<String>,
    explorer_gateways: Vec<PrettyDetailedGatewayBond>,
    skimmed_gateways: Vec<SkimmedNode>,
) -> anyhow::Result<Vec<GatewayRecord>> {
    let mut gateway_records = Vec::new();

    for gateway in gateways {
        let identity_key = gateway.bond.identity();
        let bonded = true;
        let last_updated_utc = chrono::offset::Utc::now().timestamp();
        let blacklisted = gateways_blacklisted.contains(identity_key);

        let self_described = gateway
            .self_described
            .as_ref()
            .and_then(|v| serde_json::to_string(&v).ok());

        let explorer_pretty_bond = explorer_gateways
            .iter()
            .find(|g| g.gateway.identity_key.eq(identity_key));
        let explorer_pretty_bond = explorer_pretty_bond.and_then(|g| serde_json::to_string(g).ok());

        let performance = skimmed_gateways
            .iter()
            .find(|g| g.ed25519_identity_pubkey.eq(identity_key))
            .map(|g| g.performance)
            .unwrap_or_default()
            .round_to_integer();

        gateway_records.push(GatewayRecord {
            identity_key: identity_key.to_owned(),
            bonded,
            blacklisted,
            self_described,
            explorer_pretty_bond,
            last_updated_utc,
            performance,
        });
    }

    Ok(gateway_records)
}

fn prepare_mixnode_data(
    mixnodes: &[MixNodeBondAnnotated],
    mixnodes_described: Vec<DescribedMixNode>,
    delegation_program_members: Vec<u32>,
) -> anyhow::Result<Vec<MixnodeRecord>> {
    let mut mixnode_records = Vec::new();

    for mixnode in mixnodes {
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

        let last_updated_utc = chrono::offset::Utc::now().timestamp();

        mixnode_records.push(MixnodeRecord {
            mix_id,
            identity_key: identity_key.to_owned(),
            bonded,
            total_stake,
            host,
            http_port,
            blacklisted,
            full_details,
            self_described,
            last_updated_utc,
            is_dp_delegatee,
        });
    }

    Ok(mixnode_records)
}

async fn calculate_stats(pool: &DbPool) -> anyhow::Result<(usize, usize)> {
    let mut conn = pool.acquire().await?;

    let all_historical_gateways = sqlx::query_scalar!(r#"SELECT count(id) FROM gateways"#)
        .fetch_one(&mut *conn)
        .await? as usize;

    let all_historical_mixnodes = sqlx::query_scalar!(r#"SELECT count(id) FROM mixnodes"#)
        .fetch_one(&mut *conn)
        .await? as usize;

    Ok((all_historical_gateways, all_historical_mixnodes))
}

async fn get_delegation_program_details(
    network_details: &NymNetworkDetails,
) -> anyhow::Result<Vec<u32>> {
    let config = nym_validator_client::nyxd::Config::try_from_nym_network_details(network_details)?;

    // TODO dz should this be configurable?
    let client = NyxdClient::connect(config, "https://rpc.nymtech.net")
        .map_err(|err| anyhow::anyhow!("Couldn't connect: {}", err))?;

    let account_id = AccountId::from_str(DELEGATION_PROGRAM_WALLET)
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
