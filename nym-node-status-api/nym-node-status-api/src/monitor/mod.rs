#![allow(deprecated)]

use crate::db::models::{
    gateway, mixnode, GatewayInsertRecord, MixnodeRecord, NetworkSummary, NymNodeInsertRecord,
    ASSIGNED_ENTRY_COUNT, ASSIGNED_EXIT_COUNT, ASSIGNED_MIXING_COUNT, GATEWAYS_BONDED_COUNT,
    GATEWAYS_HISTORICAL_COUNT, MIXNODES_HISTORICAL_COUNT, MIXNODES_LEGACY_COUNT,
    NYMNODES_DESCRIBED_COUNT, NYMNODE_COUNT,
};
use crate::db::{queries, DbPool};
use crate::utils::now_utc;
use crate::utils::{decimal_to_i64, LogError, NumericalCheckedCast};
use anyhow::anyhow;
use moka::future::Cache;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::{
    client::{NodeId, NymApiClientExt, NymNodeDetails},
    models::{LegacyDescribedMixNode, MixNodeBondAnnotated, NymNodeDescription},
};
use nym_validator_client::{
    nym_nodes::{NodeRole, SkimmedNode},
    nyxd::{contract_traits::PagedMixnetQueryClient, AccountId},
    NymApiClient, QueryHttpRpcNyxdClient,
};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
};
use tokio::{sync::RwLock, time::Duration};
use tracing::instrument;

pub(crate) use geodata::{ExplorerPrettyBond, IpInfoClient, Location};
pub(crate) use node_delegations::DelegationsCache;

mod geodata;
mod node_delegations;

const MONITOR_FAILURE_RETRY_DELAY: Duration = Duration::from_secs(60);
static DELEGATION_PROGRAM_WALLET: &str = "n1rnxpdpx3kldygsklfft0gech7fhfcux4zst5lw";
pub(crate) type NodeGeoCache = Cache<NodeId, Location>;

struct Monitor {
    db_pool: DbPool,
    network_details: NymNetworkDetails,
    nym_api_client_timeout: Duration,
    nyxd_client: QueryHttpRpcNyxdClient,
    ipinfo: IpInfoClient,
    geocache: NodeGeoCache,
    node_delegations: Arc<RwLock<DelegationsCache>>,
}

// TODO dz: query many NYM APIs:
// multiple instances running directory cache, ask sachin
#[instrument(level = "debug", name = "data_monitor", skip_all)]
pub(crate) async fn spawn_in_background(
    db_pool: DbPool,
    nym_api_client_timeout: Duration,
    nyxd_client: nym_validator_client::QueryHttpRpcNyxdClient,
    refresh_interval: Duration,
    ipinfo_api_token: String,
    geocache: NodeGeoCache,
    node_delegations: Arc<RwLock<DelegationsCache>>,
) {
    let ipinfo = IpInfoClient::new(ipinfo_api_token.clone());

    let mut monitor = Monitor {
        db_pool,
        network_details: nym_network_defaults::NymNetworkDetails::new_from_env(),
        nym_api_client_timeout,
        nyxd_client,
        ipinfo,
        geocache,
        node_delegations,
    };

    loop {
        tracing::info!("Refreshing node info...");

        if let Err(e) = monitor.run().await {
            tracing::error!(
                "Monitor run failed: {e}, retrying in {}s...",
                MONITOR_FAILURE_RETRY_DELAY.as_secs()
            );
            tokio::time::sleep(MONITOR_FAILURE_RETRY_DELAY).await;
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

        let nym_api = nym_http_api_client::ClientBuilder::new_with_url(default_api_url)
            .no_hickory_dns()
            .with_timeout(self.nym_api_client_timeout)
            .build::<&str>()?;

        let api_client = NymApiClient::from(nym_api);

        let described_nodes = api_client
            .get_all_described_nodes()
            .await
            .log_error("get_all_described_nodes")?
            .into_iter()
            .map(|elem| (elem.node_id, elem))
            .collect::<HashMap<_, _>>();
        tracing::info!("🟣 described nodes: {}", described_nodes.len());

        let gateways = described_nodes
            .iter()
            .filter_map(|(_, node)| {
                if node.description.declared_role.entry || node.description.declared_role.exit_ipr {
                    Some(node)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        tracing::info!("🟣 🚪 gateway nodes: {}", gateways.len());

        let bonded_nym_nodes = api_client
            .get_all_bonded_nym_nodes()
            .await?
            .into_iter()
            .map(|node| (node.bond_information.node_id, node))
            // for faster reads
            .collect::<HashMap<_, _>>();

        tracing::info!("🟣 bonded_nodes: {}", bonded_nym_nodes.len());

        // returns only bonded nodes
        let nym_nodes = api_client
            .get_all_basic_nodes()
            .await
            .log_error("get_all_basic_nodes")?;

        let nym_node_count = nym_nodes.len();
        tracing::info!("🟣 get_all_basic_nodes: {}", nym_node_count);

        let nym_node_records =
            self.prepare_nym_node_data(nym_nodes.clone(), &bonded_nym_nodes, &described_nodes);
        queries::update_nym_nodes(&self.db_pool, nym_node_records)
            .await
            .map(|inserted| {
                tracing::debug!("{} nym nodes written to DB!", inserted);
            })?;

        // refresh geodata for all nodes
        for node_description in described_nodes.values() {
            self.location_cached(node_description).await;
        }

        let mixnodes_detailed = api_client
            .nym_api
            .get_mixnodes_detailed_unfiltered()
            .await
            .log_error("get_mixnodes_detailed_unfiltered")?;

        tracing::info!(
            "🟣 mixnodes_detailed_unfiltered: {}",
            mixnodes_detailed.len()
        );

        let mixnodes_detailed_set = mixnodes_detailed
            .iter()
            .map(|elem| elem.identity_key().to_owned())
            .collect::<HashSet<_>>();

        let mixnodes_legacy = nym_nodes
            .iter()
            .filter(|node| {
                mixnodes_detailed_set.contains(&node.ed25519_identity_pubkey.to_base58_string())
            })
            .collect::<Vec<_>>();

        let mixnodes_described = api_client
            .nym_api
            .get_mixnodes_described()
            .await
            .log_error("get_mixnodes_described")?;

        tracing::info!("🟣 mixnodes_described: {}", mixnodes_described.len());
        let mixing_assigned_nodes = api_client
            .nym_api
            .get_basic_active_mixing_assigned_nodes(false, None, None, false)
            .await
            .log_error("get_basic_active_mixing_assigned_nodes")?
            .nodes
            .data;

        let delegation_program_members = self.get_delegation_program_details().await?;

        // keep stats for later
        let assigned_entry_count = nym_nodes
            .iter()
            .filter(|elem| matches!(elem.role, NodeRole::EntryGateway))
            .count();
        let assigned_exit_count = nym_nodes
            .iter()
            .filter(|elem| matches!(elem.role, NodeRole::ExitGateway))
            .count();
        let count_bonded_gateways = gateways.len();
        let assigned_mixing_count = mixing_assigned_nodes.len();
        let count_legacy_mixnodes = mixnodes_legacy.len();

        let gateway_records = self
            .prepare_gateway_data(&gateways, &nym_nodes, &bonded_nym_nodes)
            .await?;

        let pool = self.db_pool.clone();
        let gateways_count = gateway_records.len();
        queries::update_bonded_gateways(&pool, gateway_records)
            .await
            .map(|_| {
                tracing::debug!("{} gateway records written to DB!", gateways_count);
            })?;

        let mixnode_records = self.prepare_mixnode_data(
            &mixnodes_detailed,
            mixnodes_described,
            delegation_program_members,
        )?;
        let mixnodes_count = mixnode_records.len();
        queries::update_mixnodes(&pool, mixnode_records)
            .await
            .map(|_| {
                tracing::debug!("{} mixnode info written to DB!", mixnodes_count);
            })?;

        self.refresh_node_delegations(&bonded_nym_nodes).await;

        let (all_historical_gateways, all_historical_mixnodes) = historical_count(&pool).await?;

        //
        // write summary keys and values to table
        //

        let nodes_summary = vec![
            (NYMNODE_COUNT.to_string(), nym_node_count),
            (ASSIGNED_MIXING_COUNT.to_string(), assigned_mixing_count),
            (MIXNODES_LEGACY_COUNT.to_string(), count_legacy_mixnodes),
            (NYMNODES_DESCRIBED_COUNT.to_string(), described_nodes.len()),
            (GATEWAYS_BONDED_COUNT.to_string(), count_bonded_gateways),
            (ASSIGNED_ENTRY_COUNT.to_string(), assigned_entry_count),
            (ASSIGNED_EXIT_COUNT.to_string(), assigned_exit_count),
            // TODO dz doesn't make sense, could make sense with historical Nym
            // Nodes if we really need this data
            (
                MIXNODES_HISTORICAL_COUNT.to_string(),
                all_historical_mixnodes,
            ),
            (
                GATEWAYS_HISTORICAL_COUNT.to_string(),
                all_historical_gateways,
            ),
        ];

        let last_updated = now_utc();
        let last_updated_utc = last_updated.unix_timestamp().to_string();
        let network_summary = NetworkSummary {
            total_nodes: nym_node_count.cast_checked()?,
            mixnodes: mixnode::MixnodeSummary {
                bonded: mixnode::MixingNodesSummary {
                    count: assigned_mixing_count.cast_checked()?,
                    self_described: described_nodes.len().cast_checked()?,
                    legacy: count_legacy_mixnodes.cast_checked()?,
                    last_updated_utc: last_updated_utc.clone(),
                },
                historical: mixnode::MixnodeSummaryHistorical {
                    count: all_historical_mixnodes.cast_checked()?,
                    last_updated_utc: last_updated_utc.clone(),
                },
            },
            gateways: gateway::GatewaySummary {
                bonded: gateway::GatewaySummaryBonded {
                    count: count_bonded_gateways.cast_checked()?,
                    entry: assigned_entry_count.cast_checked()?,
                    exit: assigned_exit_count.cast_checked()?,
                    last_updated_utc: last_updated_utc.clone(),
                },
                historical: gateway::GatewaySummaryHistorical {
                    count: all_historical_gateways.cast_checked()?,
                    last_updated_utc,
                },
            },
        };

        queries::insert_summaries(&pool, nodes_summary.clone(), network_summary, last_updated)
            .await?;

        let mut log_lines: Vec<String> = vec![];
        for (key, value) in nodes_summary.iter() {
            log_lines.push(format!("{key} = {value}"));
        }

        tracing::info!("Directory summary: \n{}", log_lines.join("\n"));

        Ok(())
    }

    #[instrument(level = "info", skip_all)]
    async fn location_cached(&mut self, node: &NymNodeDescription) -> Location {
        let node_id = node.node_id;

        match self.geocache.get(&node_id).await {
            Some(location) => return location,
            None => {
                for ip in node.description.host_information.ip_address.iter() {
                    match self.ipinfo.locate_ip(ip.to_string()).await {
                        Ok(location) => {
                            self.geocache.insert(node_id, location.clone()).await;
                            return location;
                        }
                        Err(err) => {
                            tracing::warn!("Couldn't locate IP {} due to: {}", ip, err)
                        }
                    }
                }
                // if no data could be retrieved
                tracing::debug!("No geodata could be retrieved for {}", node_id);
                Location::empty()
            }
        }
    }

    fn prepare_nym_node_data(
        &self,
        skimmed_nodes: Vec<SkimmedNode>,
        bonded_node_info: &HashMap<NodeId, NymNodeDetails>,
        described_nodes: &HashMap<NodeId, NymNodeDescription>,
    ) -> Vec<NymNodeInsertRecord> {
        skimmed_nodes
            .into_iter()
            .filter_map(|skimmed_node| {
                let node_id = skimmed_node.node_id;
                let bond_info = bonded_node_info.get(&skimmed_node.node_id);
                let self_described = described_nodes.get(&skimmed_node.node_id);
                match NymNodeInsertRecord::new(skimmed_node, bond_info, self_described) {
                    Ok(record) => Some(record),
                    Err(err) => {
                        tracing::error!(
                            "Failed to create insert record for node {}: {}",
                            node_id,
                            err
                        );
                        None
                    }
                }
            })
            .collect::<Vec<_>>()
    }

    async fn prepare_gateway_data(
        &mut self,
        described_gateways: &[&NymNodeDescription],
        skimmed_gateways: &[SkimmedNode],
        bonded_nodes: &HashMap<NodeId, NymNodeDetails>,
    ) -> anyhow::Result<Vec<GatewayInsertRecord>> {
        let mut gateway_records = Vec::new();

        for gateway in described_gateways {
            let identity_key = gateway.ed25519_identity_key().to_base58_string();
            let bonded = bonded_nodes.contains_key(&gateway.node_id);
            let last_updated_utc = now_utc().unix_timestamp();

            let self_described = serde_json::to_string(&gateway.description)?;

            let explorer_pretty_bond = {
                let location = self.location_cached(gateway).await;
                bonded_nodes
                    .get(&gateway.node_id)
                    .map(|details| ExplorerPrettyBond {
                        identity_key: gateway.ed25519_identity_key().to_base58_string(),
                        owner: details.bond_information.owner.to_owned(),
                        pledge_amount: details.bond_information.original_pledge.to_owned(),
                        location,
                    })
            };
            let explorer_pretty_bond =
                explorer_pretty_bond.and_then(|g| serde_json::to_string(&g).ok());

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

            gateway_records.push(GatewayInsertRecord {
                identity_key: identity_key.to_owned(),
                bonded,
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
            // only bonded nodes are given to this function
            let bonded = true;
            let total_stake = decimal_to_i64(mixnode.mixnode_details.total_stake());
            let node_info = mixnode.mix_node();
            let host = node_info.host.clone();
            let http_port = node_info.http_api_port;
            // Contains all the information including what's above
            let full_details = serde_json::to_string(&mixnode)?;

            let mixnode_described = mixnodes_described.iter().find(|m| m.bond.mix_id == mix_id);
            let self_described = mixnode_described.and_then(|v| serde_json::to_string(v).ok());
            let is_dp_delegatee = delegation_program_members.contains(&mix_id);

            let last_updated_utc = now_utc().unix_timestamp();

            mixnode_records.push(MixnodeRecord {
                mix_id,
                identity_key: identity_key.to_owned(),
                bonded,
                total_stake,
                host,
                http_port,
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
                tracing::info!("ipinfo monthly bandwidth: {} spent", bandwidth.month);
            }
            Err(err) => {
                tracing::debug!("Couldn't check ipinfo bandwidth: {}", err);
            }
        }
    }

    #[instrument(level = "info", skip_all)]
    async fn refresh_node_delegations(&mut self, bonded_nodes: &HashMap<NodeId, NymNodeDetails>) {
        let delegations_per_node = node_delegations::refresh(&self.nyxd_client, bonded_nodes).await;

        // update after refreshing all to avoid holding write lock for too long
        *self.node_delegations.write().await = delegations_per_node;
    }

    async fn get_delegation_program_details(&self) -> anyhow::Result<Vec<NodeId>> {
        let account_id = AccountId::from_str(DELEGATION_PROGRAM_WALLET)
            .map_err(|e| anyhow!("Invalid bech32 address: {}", e))?;

        let delegations = self
            .nyxd_client
            .get_all_delegator_delegations(&account_id)
            .await?;

        let mix_ids: Vec<NodeId> = delegations
            .iter()
            .map(|delegation| delegation.node_id)
            .collect();

        Ok(mix_ids)
    }
}

async fn historical_count(pool: &DbPool) -> anyhow::Result<(usize, usize)> {
    let mut conn = pool.acquire().await?;

    #[cfg(feature = "sqlite")]
    let all_historical_gateways = sqlx::query_scalar!(r#"SELECT count(id) FROM gateways"#)
        .fetch_one(&mut *conn)
        .await?
        .cast_checked()?;

    #[cfg(feature = "pg")]
    let all_historical_gateways = sqlx::query_scalar!(r#"SELECT count(id) FROM gateways"#)
        .fetch_one(&mut *conn)
        .await?
        .unwrap_or(0)
        .cast_checked()?;

    #[cfg(feature = "sqlite")]
    let all_historical_mixnodes = sqlx::query_scalar!(r#"SELECT count(id) FROM mixnodes"#)
        .fetch_one(&mut *conn)
        .await?
        .cast_checked()?;

    #[cfg(feature = "pg")]
    let all_historical_mixnodes = sqlx::query_scalar!(r#"SELECT count(id) FROM mixnodes"#)
        .fetch_one(&mut *conn)
        .await?
        .unwrap_or(0)
        .cast_checked()?;

    Ok((all_historical_gateways, all_historical_mixnodes))
}
