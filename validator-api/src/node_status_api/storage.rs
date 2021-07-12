// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    GatewayStatusReport, GatewayUptimeHistory, HistoricalUptime, MixnodeStatusReport,
    MixnodeUptimeHistory, NodeStatusApiError, Uptime,
};
use rocket::fairing::{self, AdHoc};
use rocket::{Build, Rocket};
use sqlx::ConnectOptions;
// use std::fmt::{self, Display, Formatter};
use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::node_status_api::utils::{ActiveNodeDayStatuses, NodeStatus};
use crate::node_status_api::ONE_DAY;
use sqlx::types::time::OffsetDateTime;
use std::convert::TryFrom;
use std::time::{SystemTime, UNIX_EPOCH};

// A type alias to be more explicit about type of timestamp used.
type UnixTimestamp = i64;

// note that clone here is fine as upon cloning the same underlying pool will be used
// the reason 'inner' was introduced was so that there would be an explicit split to
// place where pure SQL is used (i.e. `Inner` should be the only place containing any sort
// of SQL while `NodeStatusStorage` should provide a slightly higher level API)
#[derive(Clone)]
pub(crate) struct NodeStatusStorage {
    inner: NodeStatusStorageInner,
}

#[derive(Clone)]
struct NodeStatusStorageInner {
    connection_pool: sqlx::SqlitePool,
}

impl NodeStatusStorage {
    async fn init(rocket: Rocket<Build>) -> fairing::Result {
        use rocket_sync_db_pools::Config;

        let config = match Config::from("node-status-api-db", &rocket) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to read SQLx config: {}", e);
                return Err(rocket);
            }
        };

        // TODO: if needed we can inject more stuff here based on our validator-api global config
        // struct. Maybe different pool size or timeout intervals?
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&config.url)
            .create_if_missing(true);

        // TODO: do we want auto_vacuum ?

        opts.disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(e) => {
                error!("Failed to connect to SQLx database: {}", e);
                return Err(rocket);
            }
        };

        if let Err(e) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to initialize SQLx database: {}", e);
            return Err(rocket);
        }

        info!("Database migration finished!");

        let storage = NodeStatusStorage {
            inner: NodeStatusStorageInner { connection_pool },
        };

        Ok(rocket.manage(storage))
    }

    pub(crate) fn stage() -> AdHoc {
        AdHoc::try_on_ignite("SQLx Database", NodeStatusStorage::init)
    }

    /// Gets all statuses for particular mixnode (ipv4 and ipv6) that were inserted in last 24h.
    async fn get_mixnode_daily_statuses(
        &self,
        identity: &str,
    ) -> Result<(Vec<NodeStatus>, Vec<NodeStatus>), NodeStatusApiError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = now - ONE_DAY;

        let ipv4_statuses = self
            .inner
            .get_mixnode_ipv4_statuses_since(identity, day_ago.unix_timestamp())
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        let ipv6_statuses = self
            .inner
            .get_mixnode_ipv6_statuses_since(identity, day_ago.unix_timestamp())
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        Ok((ipv4_statuses, ipv6_statuses))
    }

    /// Gets all statuses for particular gateway (ipv4 and ipv6) that were inserted in last 24h.
    async fn get_gateway_daily_statuses(
        &self,
        identity: &str,
    ) -> Result<(Vec<NodeStatus>, Vec<NodeStatus>), NodeStatusApiError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = now - ONE_DAY;

        let ipv4_statuses = self
            .inner
            .get_gateway_ipv4_statuses_since(identity, day_ago.unix_timestamp())
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        let ipv6_statuses = self
            .inner
            .get_gateway_ipv6_statuses_since(identity, day_ago.unix_timestamp())
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        Ok((ipv4_statuses, ipv6_statuses))
    }

    /// Tries to construct a status report for mixnode with the specified identity.
    pub(crate) async fn construct_mixnode_report(
        &self,
        identity: &str,
    ) -> Result<MixnodeStatusReport, NodeStatusApiError> {
        let (ipv4_statuses, ipv6_statuses) = self.get_mixnode_daily_statuses(identity).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if ipv4_statuses.is_empty() {
            return Err(NodeStatusApiError::MixnodeReportNotFound(
                identity.to_owned(),
            ));
        }

        // now, technically this is not a critical error, but this should have NEVER happened in the first place
        // so something super weird is going on
        if ipv4_statuses.len() != ipv6_statuses.len() {
            error!("Somehow we have different number of ipv4 and ipv6 statuses for mixnode {}! (ipv4: {}, ipv6: {})",
            identity,
                ipv4_statuses.len(),
                ipv6_statuses.len(),
            )
        }

        Ok(MixnodeStatusReport::construct_from_last_day_reports(
            identity.to_owned(),
            "foomp".to_owned(),
            ipv4_statuses,
            ipv6_statuses,
        ))
    }

    pub(crate) async fn construct_gateway_report(
        &self,
        identity: &str,
    ) -> Result<GatewayStatusReport, NodeStatusApiError> {
        let (ipv4_statuses, ipv6_statuses) = self.get_gateway_daily_statuses(identity).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if ipv4_statuses.is_empty() {
            return Err(NodeStatusApiError::GatewayReportNotFound(
                identity.to_owned(),
            ));
        }

        // now, technically this is not a critical error, but this should have NEVER happened in the first place
        // so something super weird is going on
        if ipv4_statuses.len() != ipv6_statuses.len() {
            error!("Somehow we have different number of ipv4 and ipv6 statuses for gateway {}! (ipv4: {}, ipv6: {})",
                   identity,
                   ipv4_statuses.len(),
                   ipv6_statuses.len(),
            )
        }

        Ok(GatewayStatusReport::construct_from_last_day_reports(
            identity.to_owned(),
            "foomp".to_owned(),
            ipv4_statuses,
            ipv6_statuses,
        ))
    }

    pub(crate) async fn get_mixnode_uptime_history(
        &self,
        identity: &str,
    ) -> Result<MixnodeUptimeHistory, NodeStatusApiError> {
        let history = self
            .inner
            .get_mixnode_historical_uptimes(identity)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        if history.is_empty() {
            return Err(NodeStatusApiError::MixnodeUptimeHistoryNotFound(
                identity.to_owned(),
            ));
        }

        Ok(MixnodeUptimeHistory::new(
            identity.to_owned(),
            "foomp".to_owned(),
            history,
        ))
    }

    pub(crate) async fn get_gateway_uptime_history(
        &self,
        identity: &str,
    ) -> Result<GatewayUptimeHistory, NodeStatusApiError> {
        let history = self
            .inner
            .get_gateway_historical_uptimes(identity)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        if history.is_empty() {
            return Err(NodeStatusApiError::GatewayUptimeHistoryNotFound(
                identity.to_owned(),
            ));
        }

        Ok(GatewayUptimeHistory::new(
            identity.to_owned(),
            "foomp".to_owned(),
            history,
        ))
    }

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    pub(crate) async fn get_all_mixnode_reports(
        &self,
    ) -> Result<Vec<MixnodeStatusReport>, NodeStatusApiError> {
        let reports = self
            .inner
            .get_all_active_mixnodes_statuses()
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?
            .into_iter()
            .map(|statuses| {
                MixnodeStatusReport::construct_from_last_day_reports(
                    statuses.pub_key,
                    statuses.owner,
                    statuses.ipv4_statuses,
                    statuses.ipv6_statuses,
                )
            })
            .collect();

        Ok(reports)
    }

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    pub(crate) async fn get_all_gateway_reports(
        &self,
    ) -> Result<Vec<GatewayStatusReport>, NodeStatusApiError> {
        let reports = self
            .inner
            .get_all_active_gateways_statuses()
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?
            .into_iter()
            .map(|statuses| {
                GatewayStatusReport::construct_from_last_day_reports(
                    statuses.pub_key,
                    statuses.owner,
                    statuses.ipv4_statuses,
                    statuses.ipv6_statuses,
                )
            })
            .collect();

        Ok(reports)
    }

    // Used by network monitor
    pub(crate) async fn submit_new_statuses(
        &self,
        mixnode_results: Vec<NodeResult>,
        gateway_results: Vec<NodeResult>,
    ) -> Result<(), NodeStatusApiError> {
        self.inner
            .submit_new_statuses(mixnode_results, gateway_results)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)
    }

    // Called on timer/reward script
    pub(crate) async fn update_historical_uptimes(&self) -> Result<(), NodeStatusApiError> {
        let today_iso_8601 = OffsetDateTime::now_utc().date().to_string();

        // get statuses for all active mixnodes...
        let active_mixnodes_statuses = self
            .inner
            .get_all_active_mixnodes_statuses()
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        for statuses in active_mixnodes_statuses.into_iter() {
            let ipv4_day_up = statuses
                .ipv4_statuses
                .iter()
                .filter(|status| status.up)
                .count();
            let ipv6_day_up = statuses
                .ipv6_statuses
                .iter()
                .filter(|status| status.up)
                .count();

            // calculate their uptimes for the last 24h
            let ipv4_uptime = Uptime::from_ratio(ipv4_day_up, statuses.ipv4_statuses.len())
                .unwrap()
                .u8();
            let ipv6_uptime = Uptime::from_ratio(ipv6_day_up, statuses.ipv6_statuses.len())
                .unwrap()
                .u8();

            // and insert into the database
            self.inner
                .insert_mixnode_historical_uptime(
                    statuses.node_id,
                    &today_iso_8601,
                    ipv4_uptime,
                    ipv6_uptime,
                )
                .await
                .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;
        }

        // get statuses for all active gateways...
        let active_gateways_statuses = self
            .inner
            .get_all_active_gateways_statuses()
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        for statuses in active_gateways_statuses.into_iter() {
            let ipv4_day_up = statuses
                .ipv4_statuses
                .iter()
                .filter(|status| status.up)
                .count();
            let ipv6_day_up = statuses
                .ipv6_statuses
                .iter()
                .filter(|status| status.up)
                .count();

            // calculate their uptimes for the last 24h
            let ipv4_uptime = Uptime::from_ratio(ipv4_day_up, statuses.ipv4_statuses.len())
                .unwrap()
                .u8();
            let ipv6_uptime = Uptime::from_ratio(ipv6_day_up, statuses.ipv6_statuses.len())
                .unwrap()
                .u8();

            // and insert into the database
            self.inner
                .insert_gateway_historical_uptime(
                    statuses.node_id,
                    &today_iso_8601,
                    ipv4_uptime,
                    ipv6_uptime,
                )
                .await
                .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;
        }

        Ok(())
    }

    // Called on timer/reward script
    pub(crate) async fn purge_old_statuses(&self) -> Result<(), NodeStatusApiError> {
        self.inner
            .purge_old_statuses()
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)
    }
}

// all SQL goes here
impl NodeStatusStorageInner {
    /// Gets all ipv4 statuses for mixnode with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_mixnode_ipv4_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv4_status
                    JOIN mixnode_details
                    ON mixnode_ipv4_status.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.pub_key=? AND mixnode_ipv4_status.timestamp > ?;
            "#,
            identity,
            timestamp,
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| NodeStatus {
            timestamp: row.timestamp,
            up: row.up,
        })
        .collect();

        Ok(reports)
    }

    /// Gets all ipv6 statuses for mixnode with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_mixnode_ipv6_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv6_status
                    JOIN mixnode_details
                    ON mixnode_ipv6_status.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.pub_key=? AND mixnode_ipv6_status.timestamp > ?;
            "#,
            identity,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| NodeStatus {
            timestamp: row.timestamp,
            up: row.up,
        })
        .collect();

        Ok(reports)
    }

    /// Gets all ipv4 statuses for gateway with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_gateway_ipv4_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv4_status
                    JOIN gateway_details
                    ON gateway_ipv4_status.gateway_details_id = gateway_details.id
                    WHERE gateway_details.pub_key=? AND gateway_ipv4_status.timestamp > ?;
            "#,
            identity,
            timestamp,
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| NodeStatus {
            timestamp: row.timestamp,
            up: row.up,
        })
        .collect();

        Ok(reports)
    }

    /// Gets all ipv6 statuses for gateway with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_gateway_ipv6_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv6_status
                    JOIN gateway_details
                    ON gateway_ipv6_status.gateway_details_id = gateway_details.id
                    WHERE gateway_details.pub_key=? AND gateway_ipv6_status.timestamp > ?;
            "#,
            identity,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| NodeStatus {
            timestamp: row.timestamp,
            up: row.up,
        })
        .collect();

        Ok(reports)
    }

    /// Gets the historical daily uptime associated with the particular mixnode
    async fn get_mixnode_historical_uptimes(
        &self,
        identity: &str,
    ) -> Result<Vec<HistoricalUptime>, sqlx::Error> {
        let uptimes = sqlx::query!(
            r#"
                SELECT date, ipv4_uptime, ipv6_uptime
                    FROM mixnode_historical_uptime
                    JOIN mixnode_details
                    ON mixnode_historical_uptime.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.pub_key = ?
                    ORDER BY date ASC
            "#,
            identity
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .filter_map(|row| {
            Uptime::try_from(row.ipv4_uptime)
                .ok()
                .map(|ipv4_uptime| {
                    Uptime::try_from(row.ipv6_uptime)
                        .ok()
                        .map(|ipv6_uptime| HistoricalUptime {
                            date: row.date,
                            ipv4_uptime,
                            ipv6_uptime,
                        })
                })
                .flatten()
        })
        .collect();

        Ok(uptimes)
    }

    /// Gets the historical daily uptime associated with the particular gateway
    async fn get_gateway_historical_uptimes(
        &self,
        identity: &str,
    ) -> Result<Vec<HistoricalUptime>, sqlx::Error> {
        let uptimes = sqlx::query!(
            r#"
                SELECT date, ipv4_uptime, ipv6_uptime
                    FROM gateway_historical_uptime
                    JOIN gateway_details
                    ON gateway_historical_uptime.gateway_details_id = gateway_details.id
                    WHERE gateway_details.pub_key = ?
                    ORDER BY date ASC
            "#,
            identity
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .filter_map(|row| {
            Uptime::try_from(row.ipv4_uptime)
                .ok()
                .map(|ipv4_uptime| {
                    Uptime::try_from(row.ipv6_uptime)
                        .ok()
                        .map(|ipv6_uptime| HistoricalUptime {
                            date: row.date,
                            ipv4_uptime,
                            ipv6_uptime,
                        })
                })
                .flatten()
        })
        .collect();

        Ok(uptimes)
    }

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    /// Returns public key, owner and id of all mixnodes that have had any ipv4 statuses submitted
    /// since provided timestamp.
    async fn get_all_active_mixnodes(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<(String, String, i64)>, sqlx::Error> {
        // find mixnode details of all nodes that have had at least 1 ipv4 status since the provided
        // timestamp
        // TODO: I dont know if theres a potential issue of if we have a lot of inactive nodes that
        // haven't mixed in ages, they might increase the query times?
        let pub_keys_owners = sqlx::query!(
            r#"
                SELECT DISTINCT pub_key, owner, id
                    FROM mixnode_details
                    JOIN mixnode_ipv4_status
                    ON mixnode_details.id = mixnode_ipv4_status.mixnode_details_id
                    WHERE EXISTS (
                        SELECT 1 FROM mixnode_ipv4_status WHERE timestamp > ?
                    )
            "#,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .filter_map(|row| row.id.map(|id| (row.pub_key, row.owner, id)))
        .collect();

        Ok(pub_keys_owners)
    }

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    /// Returns public key, owner and id of all gateways that have had any ipv4 statuses submitted
    /// since provided timestamp.
    async fn get_all_active_gateways(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<(String, String, i64)>, sqlx::Error> {
        let pub_keys_owners = sqlx::query!(
            r#"
                SELECT DISTINCT pub_key, owner, id
                    FROM gateway_details
                    JOIN gateway_ipv4_status
                    ON gateway_details.id = gateway_ipv4_status.gateway_details_id
                    WHERE EXISTS (
                        SELECT 1 FROM gateway_ipv4_status WHERE timestamp > ?
                    )
            "#,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .filter_map(|row| row.id.map(|id| (row.pub_key, row.owner, id)))
        .collect();

        Ok(pub_keys_owners)
    }

    /// Gets all ipv4 statuses for mixnode with particular id that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_mixnode_ipv4_statuses_since_by_id(
        &self,
        id: i64,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv4_status
                    WHERE mixnode_details_id=? AND timestamp > ?;
            "#,
            id,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| NodeStatus {
            timestamp: row.timestamp,
            up: row.up,
        })
        .collect();

        Ok(reports)
    }

    /// Gets all ipv6 statuses for mixnode with particular id that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_mixnode_ipv6_statuses_since_by_id(
        &self,
        id: i64,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv6_status
                    WHERE mixnode_details_id=? AND timestamp > ?;
            "#,
            id,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| NodeStatus {
            timestamp: row.timestamp,
            up: row.up,
        })
        .collect();

        Ok(reports)
    }

    /// Gets all ipv4 statuses for gateway with particular id that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_gateway_ipv4_statuses_since_by_id(
        &self,
        id: i64,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv4_status
                    WHERE gateway_details_id=? AND timestamp > ?;
            "#,
            id,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| NodeStatus {
            timestamp: row.timestamp,
            up: row.up,
        })
        .collect();

        Ok(reports)
    }

    /// Gets all ipv6 statuses for gateway with particular id that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_gateway_ipv6_statuses_since_by_id(
        &self,
        id: i64,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv6_status
                    WHERE gateway_details_id=? AND timestamp > ?;
            "#,
            id,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        .map(|row| NodeStatus {
            timestamp: row.timestamp,
            up: row.up,
        })
        .collect();

        Ok(reports)
    }

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    async fn get_all_active_mixnodes_statuses(
        &self,
    ) -> Result<Vec<ActiveNodeDayStatuses>, sqlx::Error> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();

        let active_nodes = self.get_all_active_mixnodes(day_ago).await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for (pub_key, owner, id) in active_nodes.into_iter() {
            let ipv4_statuses = self
                .get_mixnode_ipv4_statuses_since_by_id(id, day_ago)
                .await?;
            let ipv6_statuses = self
                .get_mixnode_ipv6_statuses_since_by_id(id, day_ago)
                .await?;

            let statuses = ActiveNodeDayStatuses {
                pub_key,
                owner,
                node_id: id,
                ipv4_statuses,
                ipv6_statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    async fn get_all_active_gateways_statuses(
        &self,
    ) -> Result<Vec<ActiveNodeDayStatuses>, sqlx::Error> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();

        let active_nodes = self.get_all_active_gateways(day_ago).await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for (pub_key, owner, id) in active_nodes.into_iter() {
            let ipv4_statuses = self
                .get_gateway_ipv4_statuses_since_by_id(id, day_ago)
                .await?;
            let ipv6_statuses = self
                .get_gateway_ipv6_statuses_since_by_id(id, day_ago)
                .await?;

            let statuses = ActiveNodeDayStatuses {
                pub_key,
                owner,
                node_id: id,
                ipv4_statuses,
                ipv6_statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }

    /// Tries to submit [`NodeResult`] from the network monitor to the database.
    async fn submit_new_statuses(
        &self,
        mixnode_results: Vec<NodeResult>,
        gateway_results: Vec<NodeResult>,
    ) -> Result<(), sqlx::Error> {
        // TODO: lower that to debug before creating PR
        info!("Submitting new node results to the database. There are {} mixnode results and {} gateway results", mixnode_results.len(), gateway_results.len());

        let now = OffsetDateTime::now_utc().unix_timestamp();

        // insert it all in a transaction to make sure all nodes are updated at the same time
        // (plus it's a nice guard against new nodes)
        let mut tx = self.connection_pool.begin().await?;
        for mixnode_result in mixnode_results {
            // if mixnode info doesn't exist, insert it and get its id

            // TODO: the potential "problem" (if you can call it that way) is that if entry DID exist
            // then the id field will be incremented for the next node we create thus we will
            // have gaps in our ids. ask @DH if that's fine (I don't see why not because nodes
            // are still correctly ordered and you can get their total number with a simple query
            // and we'd have to run the system until the heat death of the universe to run out of id numbers)
            let mixnode_id = sqlx::query!(
                r#"
                    INSERT OR IGNORE INTO mixnode_details(pub_key, owner) VALUES (?, ?);
                    SELECT id FROM mixnode_details WHERE pub_key = ?;
                "#,
                mixnode_result.pub_key,
                mixnode_result.owner,
                mixnode_result.pub_key,
            )
            .fetch_one(&mut tx)
            .await?
            .id;

            // insert ipv4 status
            sqlx::query!(
                r#"
                    INSERT INTO mixnode_ipv4_status (mixnode_details_id, up, timestamp) VALUES (?, ?, ?);
                "#,
                mixnode_id,
                mixnode_result.working_ipv4,
                now
            )
                .execute(&mut tx)
                .await?;

            // insert ipv6 status
            sqlx::query!(
                r#"
                    INSERT INTO mixnode_ipv6_status (mixnode_details_id, up, timestamp) VALUES (?, ?, ?);
                "#,
                mixnode_id,
                mixnode_result.working_ipv6,
                now
            )
                .execute(&mut tx)
                .await?;
        }

        // repeat the procedure for gateways
        for gateway_result in gateway_results {
            // if gateway info doesn't exist, insert it and get its id

            // same ID "problem" as described for mixnode insertion
            let gateway_id = sqlx::query!(
                r#"
                    INSERT OR IGNORE INTO gateway_details(pub_key, owner) VALUES (?, ?);
                    SELECT id FROM gateway_details WHERE pub_key = ?;
                "#,
                gateway_result.pub_key,
                gateway_result.owner,
                gateway_result.pub_key,
            )
            .fetch_one(&mut tx)
            .await?
            .id;

            // insert ipv4 status
            sqlx::query!(
                r#"
                    INSERT INTO gateway_ipv4_status (gateway_details_id, up, timestamp) VALUES (?, ?, ?);
                "#,
                gateway_id,
                gateway_result.working_ipv4,
                now
            )
                .execute(&mut tx)
                .await?;

            // insert ipv6 status
            sqlx::query!(
                r#"
                    INSERT INTO gateway_ipv6_status (gateway_details_id, up, timestamp) VALUES (?, ?, ?);
                "#,
                gateway_id,
                gateway_result.working_ipv6,
                now
            )
                .execute(&mut tx)
                .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }

    async fn insert_mixnode_historical_uptime(
        &self,
        node_id: i64,
        date: &str,
        ipv4_uptime: u8,
        ipv6_uptime: u8,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("INSERT INTO mixnode_historical_uptime(mixnode_details_id, date, ipv4_uptime, ipv6_uptime) VALUES (?, ?, ?, ?)",
            node_id,
                date,
                ipv4_uptime,
                ipv6_uptime,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }

    async fn insert_gateway_historical_uptime(
        &self,
        node_id: i64,
        date: &str,
        ipv4_uptime: u8,
        ipv6_uptime: u8,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("INSERT INTO gateway_historical_uptime(gateway_details_id, date, ipv4_uptime, ipv6_uptime) VALUES (?, ?, ?, ?)",
            node_id,
                date,
                ipv4_uptime,
                ipv6_uptime,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }

    /// Removes all statuses from the database that are older than 48h.
    async fn purge_old_statuses(&self) -> Result<(), sqlx::Error> {
        let now = OffsetDateTime::now_utc();
        let two_days_ago = (now - 2 * ONE_DAY).unix_timestamp();

        sqlx::query!(
            "DELETE FROM mixnode_ipv4_status WHERE timestamp < ?",
            two_days_ago
        )
        .execute(&self.connection_pool)
        .await?;

        sqlx::query!(
            "DELETE FROM mixnode_ipv6_status WHERE timestamp < ?",
            two_days_ago
        )
        .execute(&self.connection_pool)
        .await?;

        sqlx::query!(
            "DELETE FROM gateway_ipv4_status WHERE timestamp < ?",
            two_days_ago
        )
        .execute(&self.connection_pool)
        .await?;

        sqlx::query!(
            "DELETE FROM gateway_ipv6_status WHERE timestamp < ?",
            two_days_ago
        )
        .execute(&self.connection_pool)
        .await?;

        Ok(())
    }
}
