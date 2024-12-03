#![allow(deprecated)]

use crate::db::models::{
    gateway, mixnode, GatewayRecord, MixnodeRecord, NetworkSummary, GATEWAYS_BLACKLISTED_COUNT,
    GATEWAYS_BONDED_COUNT, GATEWAYS_HISTORICAL_COUNT, MIXNODES_BLACKLISTED_COUNT,
    MIXNODES_BONDED_ACTIVE, MIXNODES_BONDED_COUNT, MIXNODES_BONDED_INACTIVE,
    MIXNODES_BONDED_RESERVE, MIXNODES_HISTORICAL_COUNT,
};
use crate::db::{queries, DbPool};
use crate::monitor::geodata::{Location, NodeGeoData};
use anyhow::anyhow;
use cosmwasm_std::Decimal;
use moka::future::Cache;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::client::{NodeId, NymApiClientExt};
use nym_validator_client::models::{
    LegacyDescribedMixNode, MixNodeBondAnnotated, NymNodeDescription,
};
use nym_validator_client::nym_nodes::SkimmedNode;
use nym_validator_client::nyxd::contract_traits::PagedMixnetQueryClient;
use nym_validator_client::nyxd::{AccountId, NyxdClient};
use nym_validator_client::NymApiClient;
use reqwest::Url;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use tokio::time::Duration;
use tracing::instrument;

pub(crate) use geodata::IpInfoClient;

mod geodata;

// TODO dz should be configurable
const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60);

static DELEGATION_PROGRAM_WALLET: &str = "n1rnxpdpx3kldygsklfft0gech7fhfcux4zst5lw";

struct Monitor {
    db_pool: DbPool,
    network_details: NymNetworkDetails,
    nym_api_client_timeout: Duration,
    nyxd_addr: Url,
    ipinfo: IpInfoClient,
    geocache: Cache<NodeId, Location>,
}

// TODO dz: query many NYM APIs:
// multiple instances running directory cache, ask sachin
#[instrument(level = "debug", name = "data_monitor", skip_all)]
pub(crate) async fn spawn_in_background(
    db_pool: DbPool,
    nym_api_client_timeout: Duration,
    nyxd_addr: Url,
    refresh_interval: Duration,
    ipinfo_api_token: String,
    geodata_ttl: Duration,
) {
    let geocache = Cache::builder().time_to_live(geodata_ttl).build();
    let ipinfo = IpInfoClient::new(ipinfo_api_token.clone());
    let mut monitor = Monitor {
        db_pool,
        network_details: nym_network_defaults::NymNetworkDetails::new_from_env(),
        nym_api_client_timeout,
        nyxd_addr,
        ipinfo,
        geocache,
    };

    loop {
        tracing::info!("Refreshing node info...");

        if let Err(e) = monitor.run().await {
            tracing::error!(
                "Monitor run failed: {e}, retrying in {}s...",
                FAILURE_RETRY_DELAY.as_secs()
            );
            // TODO dz implement some sort of backoff
            tokio::time::sleep(FAILURE_RETRY_DELAY).await;
        } else {
            tracing::info!(
                "Info successfully collected, sleeping for {}s...",
                refresh_interval.as_secs()
            );
            tokio::time::sleep(refresh_interval).await;
        }
    }
}

impl Monitor {
    async fn run(&mut self) -> anyhow::Result<()> {
        self.check_ipinfo_bandwidth().await;

        let default_api_url = self
            .network_details
            .endpoints
            .first()
            .expect("rust sdk mainnet default incorrectly configured")
            .api_url()
            .clone()
            .expect("rust sdk mainnet default missing api_url");

        let api_client =
            NymApiClient::new_with_timeout(default_api_url, self.nym_api_client_timeout);

        let all_nodes = api_client
            .get_all_described_nodes()
            .await
            .log_error("get_all_described_nodes")?;
        tracing::debug!("Fetched {} total nodes", all_nodes.len());

        let gateways = all_nodes
            .iter()
            .filter(|node| node.description.declared_role.entry)
            .collect::<Vec<_>>();
        tracing::debug!(
            "{}/{} with declared entry gateway capability",
            gateways.len(),
            all_nodes.len()
        );

        let mixnodes = all_nodes
            .iter()
            .filter(|node| node.description.declared_role.mixnode)
            .collect::<Vec<_>>();
        tracing::debug!(
            "{}/{} with declared mixnode capability",
            mixnodes.len(),
            all_nodes.len()
        );

        let bonded_node_info = api_client
            .get_all_bonded_nym_nodes()
            .await?
            .into_iter()
            .map(|node| (node.bond_information.node_id, node.bond_information))
            // for faster reads
            .collect::<HashMap<_, _>>();

        let mut gateway_geodata = Vec::new();
        for gateway in gateways.iter() {
            if let Some(node_info) = bonded_node_info.get(&gateway.node_id) {
                let gw_geodata = NodeGeoData {
                    identity_key: node_info.node.identity_key.to_owned(),
                    owner: node_info.owner.to_owned(),
                    pledge_amount: node_info.original_pledge.to_owned(),
                    location: self.location_cached(gateway).await,
                };
                gateway_geodata.push(gw_geodata);
            }
        }

        // contains performance data
        let all_skimmed_nodes = api_client
            .get_all_basic_nodes(None)
            .await
            .log_error("get_all_basic_nodes")?;

        let gateways_blacklisted = all_skimmed_nodes
            .iter()
            .filter_map(|node| {
                if node.performance.round_to_integer() <= 50 && node.supported_roles.entry {
                    Some(node.ed25519_identity_pubkey.to_base58_string())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        // Cached mixnodes don't include blacklisted nodes
        // We need that to calculate the total locked tokens later
        // TODO dz deprecated API, remove
        let legacy_mixnodes = api_client
            .nym_api
            .get_mixnodes_detailed_unfiltered()
            .await
            .log_error("get_mixnodes_detailed_unfiltered")?;
        let mixnodes_described = api_client
            .nym_api
            .get_mixnodes_described()
            .await
            .log_error("get_mixnodes_described")?;
        let mixnodes_active = api_client
            .nym_api
            .get_basic_active_mixing_assigned_nodes(None, false, None, None)
            .await
            .log_error("get_active_mixnodes")?
            .nodes
            .data;
        let delegation_program_members =
            get_delegation_program_details(&self.network_details, &self.nyxd_addr).await?;

        // keep stats for later
        let count_bonded_mixnodes = mixnodes.len();
        let count_bonded_gateways = gateways.len();
        let count_bonded_mixnodes_active = mixnodes_active.len();

        let gateway_records = self.prepare_gateway_data(
            &gateways,
            &gateways_blacklisted,
            gateway_geodata,
            all_skimmed_nodes,
        )?;

        let pool = self.db_pool.clone();
        queries::insert_gateways(&pool, gateway_records)
            .await
            .map(|_| {
                tracing::debug!("Gateway info written to DB!");
            })?;

        let count_gateways_blacklisted = gateways
            .iter()
            .filter(|gw| {
                let gw_identity = gw.ed25519_identity_key().to_base58_string();
                gateways_blacklisted.contains(&gw_identity)
            })
            .count();

        if count_gateways_blacklisted > 0 {
            queries::write_blacklisted_gateways_to_db(&pool, gateways_blacklisted.iter())
                .await
                .map(|_| {
                    tracing::debug!(
                        "Gateway blacklist info written to DB! {} blacklisted by Nym API",
                        count_gateways_blacklisted
                    )
                })?;
        }

        let mixnode_records = self.prepare_mixnode_data(
            &legacy_mixnodes,
            mixnodes_described,
            delegation_program_members,
        )?;
        queries::insert_mixnodes(&pool, mixnode_records)
            .await
            .map(|_| {
                tracing::debug!("Mixnode info written to DB!");
            })?;

        let count_mixnodes_blacklisted = legacy_mixnodes
            .iter()
            .filter(|elem| elem.blacklisted)
            .count();

        let recently_unbonded_gateways =
            queries::ensure_gateways_still_bonded(&pool, &gateways).await?;
        let recently_unbonded_mixnodes =
            queries::ensure_mixnodes_still_bonded(&pool, &legacy_mixnodes).await?;

        let count_bonded_mixnodes_reserve = 0; // TODO: NymAPI doesn't report the reserve set size
        let count_bonded_mixnodes_inactive =
            count_bonded_mixnodes.saturating_sub(count_bonded_mixnodes_active);

        let (all_historical_gateways, all_historical_mixnodes) = calculate_stats(&pool).await?;

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
            (MIXNODES_HISTORICAL_COUNT, &all_historical_mixnodes),
            (GATEWAYS_HISTORICAL_COUNT, &all_historical_gateways),
            (GATEWAYS_BLACKLISTED_COUNT, &count_gateways_blacklisted),
        ];

        let last_updated = chrono::offset::Utc::now();
        let last_updated_utc = last_updated.timestamp().to_string();
        let network_summary = NetworkSummary {
            mixnodes: mixnode::MixnodeSummary {
                bonded: mixnode::MixnodeSummaryBonded {
                    count: count_bonded_mixnodes.cast_checked()?,
                    active: count_bonded_mixnodes_active.cast_checked()?,
                    inactive: count_bonded_mixnodes_inactive.cast_checked()?,
                    reserve: count_bonded_mixnodes_reserve.cast_checked()?,
                    last_updated_utc: last_updated_utc.to_owned(),
                },
                blacklisted: mixnode::MixnodeSummaryBlacklisted {
                    count: count_mixnodes_blacklisted.cast_checked()?,
                    last_updated_utc: last_updated_utc.to_owned(),
                },
                historical: mixnode::MixnodeSummaryHistorical {
                    count: all_historical_mixnodes.cast_checked()?,
                    last_updated_utc: last_updated_utc.to_owned(),
                },
            },
            gateways: gateway::GatewaySummary {
                bonded: gateway::GatewaySummaryBonded {
                    count: count_bonded_gateways.cast_checked()?,
                    last_updated_utc: last_updated_utc.to_owned(),
                },
                blacklisted: gateway::GatewaySummaryBlacklisted {
                    count: count_gateways_blacklisted.cast_checked()?,
                    last_updated_utc: last_updated_utc.to_owned(),
                },
                historical: gateway::GatewaySummaryHistorical {
                    count: all_historical_gateways.cast_checked()?,
                    last_updated_utc: last_updated_utc.to_owned(),
                },
            },
        };

        queries::insert_summaries(&pool, &nodes_summary, &network_summary, last_updated).await?;

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

    #[instrument(level = "debug", skip_all)]
    async fn location_cached(&mut self, node: &NymNodeDescription) -> Location {
        let node_id = node.node_id;

        match self.geocache.get(&node_id).await {
            Some(location) => return location,
            None => {
                for ip in node.description.host_information.ip_address.iter() {
                    if let Ok(location) = self.ipinfo.locate_ip(ip.to_string()).await {
                        self.geocache.insert(node_id, location.clone()).await;
                        return location;
                    }
                }
                // if no data could be retrieved
                tracing::debug!("No geodata could be retrieved for {}", node_id);
                Location::empty()
            }
        }
    }

    fn prepare_gateway_data(
        &self,
        gateways: &[&NymNodeDescription],
        gateways_blacklisted: &HashSet<String>,
        gateway_geodata: Vec<NodeGeoData>,
        skimmed_gateways: Vec<SkimmedNode>,
    ) -> anyhow::Result<Vec<GatewayRecord>> {
        let mut gateway_records = Vec::new();

        for gateway in gateways {
            let identity_key = gateway.ed25519_identity_key().to_base58_string();
            let bonded = true;
            let last_updated_utc = chrono::offset::Utc::now().timestamp();
            let blacklisted = gateways_blacklisted.contains(&identity_key);

            let self_described = serde_json::to_string(&gateway.description)?;

            let explorer_pretty_bond = gateway_geodata
                .iter()
                .find(|g| g.identity_key.eq(&identity_key));
            let explorer_pretty_bond =
                explorer_pretty_bond.and_then(|g| serde_json::to_string(g).ok());

            let performance = skimmed_gateways
                .iter()
                .find(|g| {
                    g.ed25519_identity_pubkey
                        .to_base58_string()
                        .eq(&identity_key)
                })
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
        &self,
        mixnodes: &[MixNodeBondAnnotated],
        mixnodes_described: Vec<LegacyDescribedMixNode>,
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

    async fn check_ipinfo_bandwidth(&self) {
        match self.ipinfo.check_remaining_bandwidth().await {
            Ok(bandwidth) => {
                tracing::info!(
                    "ipinfo monthly bandwidth: {}/{} spent",
                    bandwidth.month,
                    bandwidth.limit
                );
            }
            Err(err) => {
                tracing::debug!("Couldn't check ipinfo bandwidth: {}", err);
            }
        }
    }
}

// TODO dz is there a common monorepo place this can be put?
pub trait NumericalCheckedCast<T>
where
    T: TryFrom<Self>,
    <T as TryFrom<Self>>::Error: std::error::Error,
    Self: std::fmt::Display + Copy,
{
    fn cast_checked(self) -> anyhow::Result<T> {
        T::try_from(self).map_err(|e| {
            anyhow::anyhow!(
                "Couldn't cast {} to {}: {}",
                self,
                std::any::type_name::<T>(),
                e
            )
        })
    }
}

impl<T, U> NumericalCheckedCast<U> for T
where
    U: TryFrom<T>,
    <U as TryFrom<T>>::Error: std::error::Error,
    T: std::fmt::Display + Copy,
{
}

async fn calculate_stats(pool: &DbPool) -> anyhow::Result<(usize, usize)> {
    let mut conn = pool.acquire().await?;

    let all_historical_gateways = sqlx::query_scalar!(r#"SELECT count(id) FROM gateways"#)
        .fetch_one(&mut *conn)
        .await?
        .cast_checked()?;

    let all_historical_mixnodes = sqlx::query_scalar!(r#"SELECT count(id) FROM mixnodes"#)
        .fetch_one(&mut *conn)
        .await?
        .cast_checked()?;

    Ok((all_historical_gateways, all_historical_mixnodes))
}

async fn get_delegation_program_details(
    network_details: &NymNetworkDetails,
    nyxd_addr: &Url,
) -> anyhow::Result<Vec<u32>> {
    let config = nym_validator_client::nyxd::Config::try_from_nym_network_details(network_details)?;

    let client = NyxdClient::connect(config, nyxd_addr.as_str())
        .map_err(|err| anyhow::anyhow!("Couldn't connect: {}", err))?;

    let account_id = AccountId::from_str(DELEGATION_PROGRAM_WALLET)
        .map_err(|e| anyhow!("Invalid bech32 address: {}", e))?;

    let delegations = client.get_all_delegator_delegations(&account_id).await?;

    let mix_ids: Vec<u32> = delegations
        .iter()
        .map(|delegation| delegation.node_id)
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

trait LogError<T, E> {
    fn log_error(self, msg: &str) -> Result<T, E>;
}

impl<T, E> LogError<T, E> for anyhow::Result<T, E>
where
    E: std::error::Error,
{
    fn log_error(self, msg: &str) -> Result<T, E> {
        if let Err(e) = &self {
            tracing::error!("[{msg}]:\t{e}");
        }
        self
    }
}
