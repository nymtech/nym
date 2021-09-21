// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::node_status_api::models::{
    GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport, MixnodeUptimeHistory,
    NodeStatusApiError, Uptime,
};
use crate::node_status_api::{ONE_DAY, ONE_HOUR};
use crate::storage::manager::StorageManager;
use crate::storage::models::NodeStatus;
use rocket::fairing::{self, AdHoc};
use rocket::{Build, Rocket};
use sqlx::types::time::OffsetDateTime;
use sqlx::ConnectOptions;
use std::path::PathBuf;

pub(crate) mod manager;
pub(crate) mod models;

// A type alias to be more explicit about type of timestamp used.
type UnixTimestamp = i64;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct NodeStatusStorage {
    manager: StorageManager,
}

impl NodeStatusStorage {
    async fn init(rocket: Rocket<Build>, database_path: PathBuf) -> fairing::Result {
        // TODO: we can inject here more stuff based on our validator-api global config
        // struct. Maybe different pool size or timeout intervals?
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&database_path)
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
            manager: StorageManager { connection_pool },
        };

        Ok(rocket.manage(storage))
    }

    pub(crate) fn stage(database_path: PathBuf) -> AdHoc {
        AdHoc::try_on_ignite("SQLx Database", |rocket| {
            NodeStatusStorage::init(rocket, database_path)
        })
    }

    /// Gets all statuses for particular mixnode (ipv4 and ipv6) that were inserted
    /// since the provided timestamp.
    ///
    /// Returns tuple containing vectors of ipv4 statuses and ipv6 statuses.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity key of the mixnode to query.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    async fn get_mixnode_statuses(
        &self,
        identity: &str,
        since: UnixTimestamp,
    ) -> Result<(Vec<NodeStatus>, Vec<NodeStatus>), NodeStatusApiError> {
        let ipv4_statuses = self
            .manager
            .get_mixnode_ipv4_statuses_since(identity, since)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        let ipv6_statuses = self
            .manager
            .get_mixnode_ipv6_statuses_since(identity, since)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        Ok((ipv4_statuses, ipv6_statuses))
    }

    /// Gets all statuses for particular gateway (ipv4 and ipv6) that were inserted
    /// since the provided timestamp.
    ///
    /// Returns tuple containing vectors of ipv4 statuses and ipv6 statuses.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity key of the gateway to query.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    async fn get_gateway_statuses(
        &self,
        identity: &str,
        since: UnixTimestamp,
    ) -> Result<(Vec<NodeStatus>, Vec<NodeStatus>), NodeStatusApiError> {
        let ipv4_statuses = self
            .manager
            .get_gateway_ipv4_statuses_since(identity, since)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        let ipv6_statuses = self
            .manager
            .get_gateway_ipv6_statuses_since(identity, since)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        Ok((ipv4_statuses, ipv6_statuses))
    }

    /// Tries to construct a status report for mixnode with the specified identity.
    pub(crate) async fn construct_mixnode_report(
        &self,
        identity: &str,
    ) -> Result<MixnodeStatusReport, NodeStatusApiError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        let (ipv4_statuses, ipv6_statuses) = self.get_mixnode_statuses(identity, day_ago).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if ipv4_statuses.is_empty() {
            return Err(NodeStatusApiError::MixnodeReportNotFound(
                identity.to_owned(),
            ));
        }

        // determine the number of runs the mixnode should have been online for
        let last_hour_runs_count = self
            .get_monitor_runs_count(hour_ago, now.unix_timestamp())
            .await?;
        let last_day_runs_count = self
            .get_monitor_runs_count(day_ago, now.unix_timestamp())
            .await?;

        // now, technically this is not a critical error, but this should have NEVER happened in the first place
        // so something super weird is going on
        if ipv4_statuses.len() != ipv6_statuses.len() {
            error!("Somehow we have different number of ipv4 and ipv6 statuses for mixnode {}! (ipv4: {}, ipv6: {})",
                   identity,
                   ipv4_statuses.len(),
                   ipv6_statuses.len(),
            )
        }

        let mixnode_owner = self
            .manager
            .get_mixnode_owner(identity)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?
            .expect("The node doesn't have an owner even though we have status information on it!");

        Ok(MixnodeStatusReport::construct_from_last_day_reports(
            now,
            identity.to_owned(),
            mixnode_owner,
            ipv4_statuses,
            ipv6_statuses,
            last_hour_runs_count,
            last_day_runs_count,
        ))
    }

    pub(crate) async fn construct_gateway_report(
        &self,
        identity: &str,
    ) -> Result<GatewayStatusReport, NodeStatusApiError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        let (ipv4_statuses, ipv6_statuses) = self.get_gateway_statuses(identity, day_ago).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if ipv4_statuses.is_empty() {
            return Err(NodeStatusApiError::GatewayReportNotFound(
                identity.to_owned(),
            ));
        }

        // determine the number of runs the gateway should have been online for
        let last_hour_runs_count = self
            .get_monitor_runs_count(hour_ago, now.unix_timestamp())
            .await?;
        let last_day_runs_count = self
            .get_monitor_runs_count(day_ago, now.unix_timestamp())
            .await?;

        // now, technically this is not a critical error, but this should have NEVER happened in the first place
        // so something super weird is going on
        if ipv4_statuses.len() != ipv6_statuses.len() {
            error!("Somehow we have different number of ipv4 and ipv6 statuses for gateway {}! (ipv4: {}, ipv6: {})",
                   identity,
                   ipv4_statuses.len(),
                   ipv6_statuses.len(),
            )
        }

        let gateway_owner = self
            .manager
            .get_gateway_owner(identity)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?
            .expect(
                "The gateway doesn't have an owner even though we have status information on it!",
            );

        Ok(GatewayStatusReport::construct_from_last_day_reports(
            now,
            identity.to_owned(),
            gateway_owner,
            ipv4_statuses,
            ipv6_statuses,
            last_hour_runs_count,
            last_day_runs_count,
        ))
    }

    pub(crate) async fn get_mixnode_uptime_history(
        &self,
        identity: &str,
    ) -> Result<MixnodeUptimeHistory, NodeStatusApiError> {
        let history = self
            .manager
            .get_mixnode_historical_uptimes(identity)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        if history.is_empty() {
            return Err(NodeStatusApiError::MixnodeUptimeHistoryNotFound(
                identity.to_owned(),
            ));
        }

        let mixnode_owner = self
            .manager
            .get_mixnode_owner(identity)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?
            .expect("The node doesn't have an owner even though we have uptime history for it!");

        Ok(MixnodeUptimeHistory::new(
            identity.to_owned(),
            mixnode_owner,
            history,
        ))
    }

    pub(crate) async fn get_gateway_uptime_history(
        &self,
        identity: &str,
    ) -> Result<GatewayUptimeHistory, NodeStatusApiError> {
        let history = self
            .manager
            .get_gateway_historical_uptimes(identity)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        if history.is_empty() {
            return Err(NodeStatusApiError::GatewayUptimeHistoryNotFound(
                identity.to_owned(),
            ));
        }

        let gateway_owner = self
            .manager
            .get_gateway_owner(identity)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?
            .expect("The gateway doesn't have an owner even though we have uptime history for it!");

        Ok(GatewayUptimeHistory::new(
            identity.to_owned(),
            gateway_owner,
            history,
        ))
    }

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    pub(crate) async fn get_all_mixnode_reports(
        &self,
    ) -> Result<Vec<MixnodeStatusReport>, NodeStatusApiError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        // determine the number of runs the mixnodes should have been online for
        let last_hour_runs_count = self
            .get_monitor_runs_count(hour_ago, now.unix_timestamp())
            .await?;
        let last_day_runs_count = self
            .get_monitor_runs_count(day_ago, now.unix_timestamp())
            .await?;

        let reports = self
            .manager
            .get_all_active_mixnodes_statuses(day_ago)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?
            .into_iter()
            .map(|statuses| {
                MixnodeStatusReport::construct_from_last_day_reports(
                    now,
                    statuses.identity,
                    statuses.owner,
                    statuses.ipv4_statuses,
                    statuses.ipv6_statuses,
                    last_hour_runs_count,
                    last_day_runs_count,
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
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        // determine the number of runs the gateways should have been online for
        let last_hour_runs_count = self
            .get_monitor_runs_count(hour_ago, now.unix_timestamp())
            .await?;
        let last_day_runs_count = self
            .get_monitor_runs_count(day_ago, now.unix_timestamp())
            .await?;

        let reports = self
            .manager
            .get_all_active_gateways_statuses(day_ago)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?
            .into_iter()
            .map(|statuses| {
                GatewayStatusReport::construct_from_last_day_reports(
                    now,
                    statuses.identity,
                    statuses.owner,
                    statuses.ipv4_statuses,
                    statuses.ipv6_statuses,
                    last_hour_runs_count,
                    last_day_runs_count,
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
        info!("Submitting new node results to the database. There are {} mixnode results and {} gateway results", mixnode_results.len(), gateway_results.len());

        let now = OffsetDateTime::now_utc().unix_timestamp();

        self.manager
            .submit_mixnode_statuses(now, mixnode_results)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        self.manager
            .submit_gateway_statuses(now, gateway_results)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)
    }

    /// Inserts an entry to the database with the network monitor test run information
    /// that has occurred at this instant.
    pub(crate) async fn insert_monitor_run(&self) -> Result<(), NodeStatusApiError> {
        let now = OffsetDateTime::now_utc().unix_timestamp();

        self.manager
            .insert_monitor_run(now)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)
    }

    /// Obtains number of network monitor test runs that have occurred within the specified interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_monitor_runs_count(
        &self,
        since: UnixTimestamp,
        until: UnixTimestamp,
    ) -> Result<usize, NodeStatusApiError> {
        let run_count = self
            .manager
            .get_monitor_runs_count(since, until)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;

        if run_count < 0 {
            // I don't think it's ever possible for SQL to return a negative value from COUNT?
            return Err(NodeStatusApiError::InternalDatabaseError);
        }
        Ok(run_count as usize)
    }

    // Called on timer/reward script
    async fn update_historical_uptimes(
        &self,
        today_iso_8601: &str,
    ) -> Result<(), NodeStatusApiError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();

        // get statuses for all active mixnodes...
        let active_mixnodes_statuses = self
            .manager
            .get_all_active_mixnodes_statuses(day_ago)
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
            self.manager
                .insert_mixnode_historical_uptime(
                    statuses.node_id,
                    today_iso_8601,
                    ipv4_uptime,
                    ipv6_uptime,
                )
                .await
                .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;
        }

        // get statuses for all active gateways...
        let active_gateways_statuses = self
            .manager
            .get_all_active_gateways_statuses(day_ago)
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
            self.manager
                .insert_gateway_historical_uptime(
                    statuses.node_id,
                    today_iso_8601,
                    ipv4_uptime,
                    ipv6_uptime,
                )
                .await
                .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;
        }

        Ok(())
    }

    async fn check_if_historical_uptimes_exist_for_date(
        &self,
        date_iso_8601: &str,
    ) -> Result<bool, NodeStatusApiError> {
        self.manager
            .check_for_historical_uptime_existence(date_iso_8601)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)
    }

    // Called on timer/reward script
    async fn purge_old_statuses(&self) -> Result<(), NodeStatusApiError> {
        let now = OffsetDateTime::now_utc();
        let two_days_ago = (now - 2 * ONE_DAY).unix_timestamp();

        self.manager
            .purge_old_mixnode_ipv4_statuses(two_days_ago)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;
        self.manager
            .purge_old_mixnode_ipv6_statuses(two_days_ago)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;
        self.manager
            .purge_old_gateway_ipv4_statuses(two_days_ago)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)?;
        self.manager
            .purge_old_gateway_ipv6_statuses(two_days_ago)
            .await
            .map_err(|_| NodeStatusApiError::InternalDatabaseError)
    }

    pub(crate) async fn daily_chores(&self) -> Result<bool, NodeStatusApiError> {
        let today_iso_8601 = OffsetDateTime::now_utc().date().to_string();

        // if we have already performed the update for today's date, don't do anything
        if self
            .check_if_historical_uptimes_exist_for_date(&today_iso_8601)
            .await?
        {
            Ok(false)
        } else {
            info!(
                "Updating historical daily uptimes of all nodes and purging old status reports..."
            );
            self.update_historical_uptimes(&today_iso_8601).await?;
            self.purge_old_statuses().await?;
            Ok(true)
        }
    }
}
