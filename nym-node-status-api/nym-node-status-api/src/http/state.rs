use cosmwasm_std::Decimal;
use itertools::Itertools;
use moka::{Entry, future::Cache};
use nym_bin_common::bin_info_owned;
use nym_contracts_common::NaiveFloat;
use nym_crypto::asymmetric::ed25519::PublicKey;
use nym_mixnet_contract_common::NodeId;
use nym_node_status_client::auth::VerifiableRequest;
use nym_validator_client::nym_api::SkimmedNodeV1;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use time::UtcDateTime;
use tokio::sync::{Mutex, MutexGuard, RwLock};
use tracing::{error, instrument, trace, warn};
use utoipa::ToSchema;

use super::models::SessionStats;
use crate::{
    db::{DbPool, queries},
    http::{
        error::{HttpError, HttpResult},
        models::{
            DVpnGateway, DailyStats, ExtendedNymNode, Gateway, NodeGeoData, SummaryHistory,
            gw_probe::socks5_calc::calculate_socks5_percentiles,
        },
    },
    monitor::{DelegationsCache, NodeGeoCache},
};

use crate::ticketbook_manager::state::TicketbookManagerState;
pub(crate) use nym_validator_client::models::BinaryBuildInformationOwned;

#[derive(Clone)]
pub(crate) struct AppState {
    db_pool: DbPool,
    cache: HttpCache,
    agent_key_list: Vec<PublicKey>,
    agent_max_count: i64,
    agent_request_freshness_requirement: time::Duration,
    node_geocache: NodeGeoCache,
    node_delegations: Arc<RwLock<DelegationsCache>>,
    bin_info: BinaryInfo,
    ticketbook_manager_state: TicketbookManagerState,
    testrun_assignment_guard: Arc<Mutex<()>>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        db_pool: DbPool,
        cache_ttl: u64,
        agent_key_list: Vec<PublicKey>,
        agent_max_count: i64,
        agent_request_freshness_requirement: time::Duration,
        node_geocache: NodeGeoCache,
        node_delegations: Arc<RwLock<DelegationsCache>>,
        ticketbook_manager_state: TicketbookManagerState,
    ) -> Self {
        Self {
            db_pool,
            cache: HttpCache::new(cache_ttl).await,
            agent_key_list,
            agent_max_count,
            agent_request_freshness_requirement,
            node_geocache,
            node_delegations,
            bin_info: BinaryInfo::new(),
            ticketbook_manager_state,
            testrun_assignment_guard: Arc::new(Default::default()),
        }
    }

    pub(crate) fn db_pool(&self) -> &DbPool {
        &self.db_pool
    }

    pub(crate) fn cache(&self) -> &HttpCache {
        &self.cache
    }

    pub(crate) fn is_registered(&self, agent_pubkey: &PublicKey) -> bool {
        self.agent_key_list.contains(agent_pubkey)
    }

    pub(crate) async fn lock_testrun_assignment(&self) -> MutexGuard<'_, ()> {
        self.testrun_assignment_guard.lock().await
    }

    pub(crate) fn agent_max_count(&self) -> i64 {
        self.agent_max_count
    }

    pub(crate) fn node_geocache(&self) -> NodeGeoCache {
        self.node_geocache.clone()
    }

    pub(crate) async fn node_delegations(
        &self,
        node_id: NodeId,
    ) -> Option<Vec<super::models::NodeDelegation>> {
        self.node_delegations
            .read()
            .await
            .delegations_owned(node_id)
    }

    pub(crate) fn health(&self) -> HealthInfo {
        let uptime = (UtcDateTime::now() - self.bin_info.startup_time).whole_seconds();
        HealthInfo { uptime }
    }

    pub(crate) fn build_information(&self) -> &BinaryBuildInformationOwned {
        &self.bin_info.build_info
    }

    pub(crate) fn ticketbook_manager_state(&self) -> &TicketbookManagerState {
        &self.ticketbook_manager_state
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) fn authenticate_agent_submission(
        &self,
        request: &impl VerifiableRequest,
    ) -> HttpResult<()> {
        if !self.is_registered(request.public_key()) {
            tracing::warn!("Public key not registered with NS API, rejecting");
            return Err(HttpError::unauthorized());
        };

        request.verify_signature().map_err(|_| {
            tracing::warn!("Signature verification failed, rejecting");
            HttpError::unauthorized()
        })?;

        Ok(())
    }

    pub(crate) fn is_fresh(&self, request_time: &i64) -> HttpResult<()> {
        // if a request took longer than N minutes to reach NS API, something is very wrong
        let request_time = time::UtcDateTime::from_unix_timestamp(*request_time).map_err(|e| {
            warn!("Failed to parse request time: {e}");
            HttpError::unauthorized()
        })?;

        let cutoff_timestamp = crate::utils::now_utc() - self.agent_request_freshness_requirement;
        if request_time < cutoff_timestamp {
            warn!(
                "Request time {} is older than cutoff {} ({}s ago), rejecting",
                request_time,
                cutoff_timestamp,
                self.agent_request_freshness_requirement.whole_seconds()
            );
            return Err(HttpError::unauthorized());
        }
        Ok(())
    }
}

static GATEWAYS_LIST_KEY: &str = "gateways";
static DVPN_GATEWAYS_LIST_KEY: &str = "dvpn_gateways";
static DVPN_GATEWAY_IPS: &str = "dvpn_gateway_ips";
static NYM_NODES_LIST_KEY: &str = "nym_nodes";
static MIXSTATS_LIST_KEY: &str = "mixstats";
static SUMMARY_HISTORY_LIST_KEY: &str = "summary-history";
static SESSION_STATS_LIST_KEY: &str = "session-stats";

const MIXNODE_STATS_HISTORY_DAYS: usize = 30;

#[derive(Debug, Clone)]
pub(crate) struct HttpCache {
    gateways: Cache<String, Arc<RwLock<Vec<Gateway>>>>,
    dvpn_gateways: Cache<String, Arc<RwLock<Vec<DVpnGateway>>>>,
    gateway_ips: Cache<String, Arc<RwLock<Vec<String>>>>,
    nym_nodes: Cache<String, Arc<RwLock<Vec<ExtendedNymNode>>>>,
    mixstats: Cache<String, Arc<RwLock<Vec<DailyStats>>>>,
    history: Cache<String, Arc<RwLock<Vec<SummaryHistory>>>>,
    session_stats: Cache<String, Arc<RwLock<Vec<SessionStats>>>>,
}

impl HttpCache {
    pub async fn new(ttl_seconds: u64) -> Self {
        HttpCache {
            gateways: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            dvpn_gateways: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            gateway_ips: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            nym_nodes: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            mixstats: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            history: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
            session_stats: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(ttl_seconds))
                .build(),
        }
    }

    pub async fn upsert_gateway_list(
        &self,
        new_gateway_list: Vec<Gateway>,
    ) -> Entry<String, Arc<RwLock<Vec<Gateway>>>> {
        self.gateways
            .entry_by_ref(GATEWAYS_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = new_gateway_list;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(new_gateway_list))
                }
            })
            .await
    }

    pub async fn get_gateway_list(&self, db: &DbPool) -> Vec<Gateway> {
        match self.gateways.get(GATEWAYS_LIST_KEY).await {
            Some(guard) => {
                tracing::trace!("Fetching from cache...");
                let read_lock = guard.read().await;
                read_lock.clone()
            }
            None => {
                // the key is missing so populate it
                tracing::trace!("No gateways in cache, refreshing cache from DB...");

                let gateways = match crate::db::queries::get_all_gateways(db).await {
                    Ok(gws) => {
                        tracing::info!("Successfully fetched {} gateways from database", gws.len());
                        if !gws.is_empty() {
                            self.upsert_gateway_list(gws.clone()).await;
                        }
                        gws
                    }
                    Err(err) => {
                        tracing::error!("CRITICAL: Failed to fetch gateways from database: {err}");
                        panic!(
                            "Cannot read gateways table - this should never happen! Error: {err}"
                        );
                    }
                };

                if gateways.is_empty() {
                    tracing::warn!("Database: gateway list is empty");
                }

                gateways
            }
        }
    }

    pub async fn upsert_dvpn_gateway_list(
        &self,
        new_gateway_list: Vec<DVpnGateway>,
    ) -> Entry<String, Arc<RwLock<Vec<DVpnGateway>>>> {
        self.dvpn_gateways
            .entry_by_ref(DVPN_GATEWAYS_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = new_gateway_list;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(new_gateway_list))
                }
            })
            .await
    }

    pub async fn get_dvpn_gateway_list(
        &self,
        db: &DbPool,
        min_node_version: &Version,
    ) -> Vec<DVpnGateway> {
        if let Some(guard) = self.dvpn_gateways.get(DVPN_GATEWAYS_LIST_KEY).await {
            tracing::trace!("Fetching from cache...");
            let read_lock = guard.read().await;
            return read_lock.clone();
        };

        tracing::info!("No gateways (dVPN) in cache, refreshing from DB...");

        let gateways = self.get_gateway_list(db).await;
        tracing::info!("Found {} gateways in database", gateways.len());

        let started_with = gateways.len();
        let skimmed_nodes = match crate::db::queries::get_described_bonded_nym_nodes(db).await {
            Ok(records) => {
                let mut nodes = HashMap::new();
                for dto in records {
                    match SkimmedNodeV1::try_from(dto) {
                        Ok(skimmed_node) => {
                            let key = skimmed_node.ed25519_identity_pubkey.to_base58_string();
                            nodes.insert(key, skimmed_node);
                        }
                        Err(err) => {
                            error!("CRITICAL: Failed to convert NymNodeDto to SkimmedNode: {err}");
                            panic!(
                                "Cannot convert database record to SkimmedNode - this should never happen! Error: {err}"
                            );
                        }
                    }
                }
                nodes
            }
            Err(err) => {
                error!("CRITICAL: Failed to query nym_nodes from database: {err}");
                panic!("Cannot read nym_nodes table - database connection issue? Error: {err}");
            }
        };

        let socks5_scores = calculate_socks5_percentiles(&gateways);

        let mut dvpd_gateways = Vec::new();
        let bonded_count = gateways.iter().filter(|gw| gw.bonded).count();

        for gw in gateways {
            let id = gw.gateway_identity_key.clone();
            // 1. reject all gateways that are not bonded
            if !gw.bonded {
                continue;
            }

            // 2. reject all gateways with zero performance
            if gw.performance == 0 {
                continue;
            }

            // 3. get corresponding directory details
            let Some(skimmed_node) = skimmed_nodes.get(&id) else {
                error!(
                    "CRITICAL: Gateway {id} exists in gateways table but not in nym_nodes table! This should not happen",
                );
                continue;
            };
            let node_id = skimmed_node.node_id;

            // 4. construct the DVpnGateway model
            let dvpn_gw = match DVpnGateway::new(gw, skimmed_node, socks5_scores.get(&id)) {
                Ok(gw) => gw,
                Err(err) => {
                    error!(
                        "CRITICAL: Failed to create DVpnGateway for node_id={node_id}, identity_key={id}: {err}",
                    );
                    // Don't panic here as this might be due to missing fields, but log it loudly
                    continue;
                }
            };

            // 5. filter out outdated nodes
            let gw_version = &dvpn_gw.build_information.build_version;
            if let Ok(gw_version) = Version::parse(gw_version) {
                if &gw_version < min_node_version {
                    continue;
                }
            } else {
                warn!("Failed to parse GW version {gw_version}");
                continue;
            }

            // 6. filter out nodes without valid country codes
            if dvpn_gw.location.two_letter_iso_country_code.len() != 2 {
                warn!(
                    "Invalid country code: {}",
                    dvpn_gw.location.two_letter_iso_country_code
                );
                continue;
            }

            dvpd_gateways.push(dvpn_gw);
        }

        // 7. finally, sort the nodes by country, then by identity key
        dvpd_gateways.sort_by_key(|item| {
            (
                item.location.two_letter_iso_country_code.clone(),
                item.identity_key.clone(),
            )
        });

        tracing::info!(
            "DVpn gateway filtering: {started_with} total gateways, {bonded_count} bonded, {} nym_nodes, {} final DVpn gateways",
            skimmed_nodes.len(),
            dvpd_gateways.len()
        );

        if dvpd_gateways.is_empty() && started_with > 0 {
            tracing::error!(
                "CRITICAL: Started with {started_with} gateways but got 0 DVpn gateways! Min version: {min_node_version}",
            );
        } else {
            tracing::info!(
                "Successfully loaded {} DVpn gateways into cache",
                dvpd_gateways.len()
            );
            self.upsert_dvpn_gateway_list(dvpd_gateways.clone()).await;
        }

        dvpd_gateways
    }

    pub async fn get_entry_dvpn_gateways(
        &self,
        db: &DbPool,
        min_node_version: &Version,
    ) -> Vec<DVpnGateway> {
        self.get_dvpn_gateway_list(db, min_node_version)
            .await
            .into_iter()
            .filter(DVpnGateway::can_route_entry)
            .collect()
    }

    pub async fn get_exit_dvpn_gateways(
        &self,
        db: &DbPool,
        min_node_version: &Version,
    ) -> Vec<DVpnGateway> {
        self.get_dvpn_gateway_list(db, min_node_version)
            .await
            .into_iter()
            .filter(DVpnGateway::can_route_exit)
            .collect()
    }

    pub async fn get_gateway_ips(&self, db: &DbPool, min_node_version: &Version) -> Vec<String> {
        match self.gateway_ips.get(DVPN_GATEWAY_IPS).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.clone()
            }
            None => {
                trace!("No exit gateway IPs in cache, refreshing...");

                let ips: Vec<String> = self
                    .get_dvpn_gateway_list(db, min_node_version)
                    .await
                    .into_iter()
                    .flat_map(|gw| gw.ip_addresses)
                    .map(|ip| ip.to_string())
                    .sorted()
                    .dedup()
                    .collect();

                self.upsert_gateway_ips(ips.clone()).await;

                ips
            }
        }
    }

    pub async fn upsert_gateway_ips(
        &self,
        ip_list: Vec<String>,
    ) -> Entry<String, Arc<RwLock<Vec<String>>>> {
        self.gateway_ips
            .entry_by_ref(DVPN_GATEWAY_IPS)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = ip_list;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(ip_list))
                }
            })
            .await
    }

    pub async fn upsert_nym_node_list(
        &self,
        nym_node_list: Vec<ExtendedNymNode>,
    ) -> Entry<String, Arc<RwLock<Vec<ExtendedNymNode>>>> {
        self.nym_nodes
            .entry_by_ref(NYM_NODES_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = nym_node_list;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(nym_node_list))
                }
            })
            .await
    }

    pub async fn get_nym_nodes_list(
        &self,
        db: &DbPool,
        node_geocache: NodeGeoCache,
    ) -> anyhow::Result<Vec<ExtendedNymNode>> {
        match self.nym_nodes.get(NYM_NODES_LIST_KEY).await {
            Some(guard) => {
                tracing::trace!("Fetching from cache...");
                let read_lock = guard.read().await;
                Ok(read_lock.clone())
            }
            None => {
                tracing::trace!("No nym nodes in cache, refreshing cache from DB...");

                let nym_nodes = aggregate_node_info_from_db(db, node_geocache).await?;

                if nym_nodes.is_empty() {
                    tracing::warn!("Database contains 0 nym nodes");
                } else {
                    self.upsert_nym_node_list(nym_nodes.clone()).await;
                }

                Ok(nym_nodes)
            }
        }
    }

    pub async fn upsert_mixnode_stats(
        &self,
        mixnode_stats: Vec<DailyStats>,
    ) -> Entry<String, Arc<RwLock<Vec<DailyStats>>>> {
        self.mixstats
            .entry_by_ref(MIXSTATS_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = mixnode_stats;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(mixnode_stats))
                }
            })
            .await
    }

    pub async fn get_mixnode_stats(&self, db: &DbPool, offset: usize) -> Vec<DailyStats> {
        let mut stats = match self.mixstats.get(MIXSTATS_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.to_vec()
            }
            None => {
                let new_node_stats = crate::db::queries::get_daily_stats(db)
                    .await
                    .inspect_err(|err| tracing::error!("{err}"))
                    // still need to return some data on API in case of internal error
                    .unwrap_or_default()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>();
                // cache result without offset
                self.upsert_mixnode_stats(new_node_stats.clone()).await;
                new_node_stats
            }
        };

        stats.truncate(MIXNODE_STATS_HISTORY_DAYS + offset);
        stats.into_iter().skip(offset).rev().collect()
    }

    pub async fn get_summary_history(&self, db: &DbPool) -> Vec<SummaryHistory> {
        match self.history.get(SUMMARY_HISTORY_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.to_vec()
            }
            None => {
                let summary_history = crate::db::queries::get_summary_history(db)
                    .await
                    .unwrap_or(vec![]);
                self.upsert_summary_history(summary_history.clone()).await;
                summary_history
            }
        }
    }

    pub async fn upsert_summary_history(
        &self,
        summary_history: Vec<SummaryHistory>,
    ) -> Entry<String, Arc<RwLock<Vec<SummaryHistory>>>> {
        self.history
            .entry_by_ref(SUMMARY_HISTORY_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = summary_history;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(summary_history))
                }
            })
            .await
    }

    pub async fn get_sessions_stats(&self, db: &DbPool) -> Vec<SessionStats> {
        match self.session_stats.get(SESSION_STATS_LIST_KEY).await {
            Some(guard) => {
                let read_lock = guard.read().await;
                read_lock.to_vec()
            }
            None => {
                let session_stats = crate::db::queries::get_sessions_stats(db)
                    .await
                    .unwrap_or_default();
                self.upsert_sessions_stats(session_stats.clone()).await;
                session_stats
            }
        }
    }

    pub async fn upsert_sessions_stats(
        &self,
        session_stats: Vec<SessionStats>,
    ) -> Entry<String, Arc<RwLock<Vec<SessionStats>>>> {
        self.session_stats
            .entry_by_ref(SESSION_STATS_LIST_KEY)
            .and_upsert_with(|maybe_entry| async {
                if let Some(entry) = maybe_entry {
                    let v = entry.into_value();
                    let mut guard = v.write().await;
                    *guard = session_stats;
                    v.clone()
                } else {
                    Arc::new(RwLock::new(session_stats))
                }
            })
            .await
    }
}

#[instrument(level = "info", skip_all)]
async fn aggregate_node_info_from_db(
    pool: &DbPool,
    node_geocache: NodeGeoCache,
) -> anyhow::Result<Vec<ExtendedNymNode>> {
    let node_bond_info = queries::get_described_node_bond_info(pool).await?;
    tracing::debug!("Described nodes with bond info: {}", node_bond_info.len());

    let skimmed_nodes = queries::get_all_nym_nodes(pool).await.map(|records| {
        records
            .into_iter()
            .filter_map(|dto| SkimmedNodeV1::try_from(dto).ok())
            .map(|skimmed_node| (skimmed_node.node_id, skimmed_node))
            .collect::<HashMap<_, _>>()
    })?;
    tracing::debug!("Skimmed nodes: {}", skimmed_nodes.len());

    let described_nodes = queries::get_node_self_description(pool).await?;
    tracing::debug!("Described nodes: {}", described_nodes.len());

    let node_descriptions = queries::get_bonded_node_description(pool).await?;

    let mut parsed_nym_nodes = Vec::new();
    for (node_id, described_node) in described_nodes {
        let bond_details = node_bond_info.get(&node_id);
        let bonded = bond_details.is_some();
        let total_stake = bond_details
            .map(|details| details.total_stake())
            .unwrap_or(Decimal::zero());
        let identity_key = described_node.ed25519_identity_key().to_string();

        let original_pledge = bond_details
            .map(|details| details.original_pledge().amount.u128())
            .unwrap_or(0u128);
        let rewarding_details = &node_bond_info
            .get(&node_id)
            .map(|details| details.rewarding_details.clone());

        let uptime = skimmed_nodes
            .get(&node_id)
            .map(|node| node.performance.naive_to_f64())
            .unwrap_or(0.0);
        let node_type = described_node.contract_node_type;
        let ip_address = described_node
            .description
            .host_information
            .ip_address
            .first()
            .map(ToString::to_string)
            .unwrap_or_default();
        let accepted_tnc = described_node
            .description
            .auxiliary_details
            .accepted_operator_terms_and_conditions;
        let self_described = described_node.description;

        let bonding_address =
            bond_details.map(|details| details.bond_information.owner.to_string());

        let node_description = node_descriptions.get(&node_id).cloned().unwrap_or_default();
        let geoip = {
            node_geocache.get(&node_id).await.map(|data| NodeGeoData {
                city: data.city,
                country: data.two_letter_iso_country_code,
                ip_address: data.ip_address,
                latitude: data.location.latitude.to_string(),
                longitude: data.location.longitude.to_string(),
                org: data.org,
                postal: data.postal,
                region: data.region,
                timezone: data.timezone,
            })
        };

        parsed_nym_nodes.push(ExtendedNymNode {
            node_id,
            identity_key,
            total_stake,
            uptime,
            ip_address,
            original_pledge,
            bonding_address,
            bonded,
            node_type,
            accepted_tnc,
            self_description: self_described,
            rewarding_details: rewarding_details.to_owned(),
            description: node_description,
            geoip,
        });
    }

    Ok(parsed_nym_nodes)
}

#[derive(Debug, Clone)]
pub(crate) struct BinaryInfo {
    startup_time: UtcDateTime,
    build_info: BinaryBuildInformationOwned,
}

impl BinaryInfo {
    fn new() -> Self {
        Self {
            startup_time: UtcDateTime::now(),
            build_info: bin_info_owned!(),
        }
    }
}

#[derive(Serialize, ToSchema, Deserialize)]
pub(crate) struct HealthInfo {
    pub(crate) uptime: i64,
}
