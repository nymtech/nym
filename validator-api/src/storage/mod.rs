// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::node_status_api::models::{
    GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport, MixnodeUptimeHistory,
    ValidatorApiStorageError,
};
use crate::node_status_api::{ONE_DAY, ONE_HOUR};
use crate::storage::manager::StorageManager;
use crate::storage::models::{
    EpochRewarding, FailedMixnodeRewardChunk, NodeStatus, PossiblyUnrewardedMixnode,
    RewardingReport,
};
use rocket::fairing::{self, AdHoc};
use rocket::{Build, Rocket};
use sqlx::ConnectOptions;
use std::path::PathBuf;
use time::OffsetDateTime;

pub(crate) mod manager;
pub(crate) mod models;

// A type alias to be more explicit about type of timestamp used.
pub(crate) type UnixTimestamp = i64;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct ValidatorApiStorage {
    manager: StorageManager,
}

impl ValidatorApiStorage {
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

        let storage = ValidatorApiStorage {
            manager: StorageManager { connection_pool },
        };

        Ok(rocket.manage(storage))
    }

    pub(crate) fn stage(database_path: PathBuf) -> AdHoc {
        AdHoc::try_on_ignite("SQLx Database", |rocket| {
            ValidatorApiStorage::init(rocket, database_path)
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
    ) -> Result<(Vec<NodeStatus>, Vec<NodeStatus>), ValidatorApiStorageError> {
        let ipv4_statuses = self
            .manager
            .get_mixnode_ipv4_statuses_since(identity, since)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        let ipv6_statuses = self
            .manager
            .get_mixnode_ipv6_statuses_since(identity, since)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

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
    ) -> Result<(Vec<NodeStatus>, Vec<NodeStatus>), ValidatorApiStorageError> {
        let ipv4_statuses = self
            .manager
            .get_gateway_ipv4_statuses_since(identity, since)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        let ipv6_statuses = self
            .manager
            .get_gateway_ipv6_statuses_since(identity, since)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        Ok((ipv4_statuses, ipv6_statuses))
    }

    /// Tries to construct a status report for mixnode with the specified identity.
    pub(crate) async fn construct_mixnode_report(
        &self,
        identity: &str,
    ) -> Result<MixnodeStatusReport, ValidatorApiStorageError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        let (ipv4_statuses, ipv6_statuses) = self.get_mixnode_statuses(identity, day_ago).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if ipv4_statuses.is_empty() {
            return Err(ValidatorApiStorageError::MixnodeReportNotFound(
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
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
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
    ) -> Result<GatewayStatusReport, ValidatorApiStorageError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        let (ipv4_statuses, ipv6_statuses) = self.get_gateway_statuses(identity, day_ago).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if ipv4_statuses.is_empty() {
            return Err(ValidatorApiStorageError::GatewayReportNotFound(
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
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
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
    ) -> Result<MixnodeUptimeHistory, ValidatorApiStorageError> {
        let history = self
            .manager
            .get_mixnode_historical_uptimes(identity)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        if history.is_empty() {
            return Err(ValidatorApiStorageError::MixnodeUptimeHistoryNotFound(
                identity.to_owned(),
            ));
        }

        let mixnode_owner = self
            .manager
            .get_mixnode_owner(identity)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
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
    ) -> Result<GatewayUptimeHistory, ValidatorApiStorageError> {
        let history = self
            .manager
            .get_gateway_historical_uptimes(identity)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        if history.is_empty() {
            return Err(ValidatorApiStorageError::GatewayUptimeHistoryNotFound(
                identity.to_owned(),
            ));
        }

        let gateway_owner = self
            .manager
            .get_gateway_owner(identity)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            .expect("The gateway doesn't have an owner even though we have uptime history for it!");

        Ok(GatewayUptimeHistory::new(
            identity.to_owned(),
            gateway_owner,
            history,
        ))
    }

    /// Obtain status reports of mixnodes that were active in the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    // NOTE: even though the arguments would suggest this function is generic in regards to
    // epoch length, the constructed reports still assume the epochs are 24h in length.
    pub(crate) async fn get_all_active_mixnode_reports_in_interval(
        &self,
        start: UnixTimestamp,
        end: UnixTimestamp,
    ) -> Result<Vec<MixnodeStatusReport>, ValidatorApiStorageError> {
        if (end - start) as u64 != ONE_DAY.as_secs() {
            warn!("Our current epoch length breaks the 24h length assumption")
        }

        let hour_ago = end - ONE_HOUR.as_secs() as i64;

        // determine the number of runs the mixnodes should have been online for
        let last_hour_runs_count = self.get_monitor_runs_count(hour_ago, end).await?;
        let last_day_runs_count = self.get_monitor_runs_count(start, end).await?;

        let reports = self
            .manager
            .get_all_active_mixnodes_statuses_in_interval(start, end)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            .into_iter()
            .map(|statuses| {
                MixnodeStatusReport::construct_from_last_day_reports(
                    OffsetDateTime::from_unix_timestamp(end).unwrap(),
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

    /// Obtain status reports of gateways that were active in the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    // NOTE: even though the arguments would suggest this function is generic in regards to
    // epoch length, the constructed reports still assume the epochs are 24h in length.
    pub(crate) async fn get_all_active_gateway_reports_in_interval(
        &self,
        start: UnixTimestamp,
        end: UnixTimestamp,
    ) -> Result<Vec<GatewayStatusReport>, ValidatorApiStorageError> {
        if (end - start) as u64 != ONE_DAY.as_secs() {
            warn!("Our current epoch length breaks the 24h length assumption")
        }

        let hour_ago = end - ONE_HOUR.as_secs() as i64;

        // determine the number of runs the mixnodes should have been online for
        let last_hour_runs_count = self.get_monitor_runs_count(hour_ago, end).await?;
        let last_day_runs_count = self.get_monitor_runs_count(start, end).await?;

        let reports = self
            .manager
            .get_all_active_gateways_statuses_in_interval(start, end)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            .into_iter()
            .map(|statuses| {
                GatewayStatusReport::construct_from_last_day_reports(
                    OffsetDateTime::from_unix_timestamp(end).unwrap(),
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
    ) -> Result<(), ValidatorApiStorageError> {
        info!("Submitting new node results to the database. There are {} mixnode results and {} gateway results", mixnode_results.len(), gateway_results.len());

        let now = OffsetDateTime::now_utc().unix_timestamp();

        self.manager
            .submit_mixnode_statuses(now, mixnode_results)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        self.manager
            .submit_gateway_statuses(now, gateway_results)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }

    /// Inserts an entry to the database with the network monitor test run information
    /// that has occurred at this instant.
    pub(crate) async fn insert_monitor_run(&self) -> Result<(), ValidatorApiStorageError> {
        let now = OffsetDateTime::now_utc().unix_timestamp();

        self.manager
            .insert_monitor_run(now)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
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
    ) -> Result<usize, ValidatorApiStorageError> {
        let run_count = self
            .manager
            .get_monitor_runs_count(since, until)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        if run_count < 0 {
            // I don't think it's ever possible for SQL to return a negative value from COUNT?
            return Err(ValidatorApiStorageError::InternalDatabaseError);
        }
        Ok(run_count as usize)
    }

    /// Given lists of reports of all monitor-active mixnodes and gateways, inserts the data into the
    /// historical uptime tables. This method is called at a 24h timer.
    ///
    /// # Arguments
    ///
    /// * `today_iso_8601`: today's date expressed in ISO 8601, i.e. YYYY-MM-DD
    /// * `mixnode_reports`: slice of reports for all monitor-active mixnodes
    /// * `gateway_reports`: slice of reports for all monitor-active gateways
    pub(crate) async fn update_historical_uptimes(
        &self,
        today_iso_8601: &str,
        mixnode_reports: &[MixnodeStatusReport],
        gateway_reports: &[GatewayStatusReport],
    ) -> Result<(), ValidatorApiStorageError> {
        for report in mixnode_reports {
            // if this ever fails, we have a super weird error because we just constructed report for that node
            // and we never delete node data!
            let node_id = match self
                .manager
                .get_mixnode_id(&report.identity)
                .await
                .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            {
                Some(node_id) => node_id,
                None => {
                    error!(
                        "Somehow we failed to grab id of mixnode {} from the database!",
                        &report.identity
                    );
                    continue;
                }
            };

            self.manager
                .insert_mixnode_historical_uptime(
                    node_id,
                    today_iso_8601,
                    report.last_day_ipv4.u8(),
                    report.last_day_ipv4.u8(),
                )
                .await
                .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;
        }

        for report in gateway_reports {
            // if this ever fails, we have a super weird error because we just constructed report for that node
            // and we never delete node data!
            let node_id = match self
                .manager
                .get_gateway_id(&report.identity)
                .await
                .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            {
                Some(node_id) => node_id,
                None => {
                    error!(
                        "Somehow we failed to grab id of gateway {} from the database!",
                        &report.identity
                    );
                    continue;
                }
            };

            self.manager
                .insert_gateway_historical_uptime(
                    node_id,
                    today_iso_8601,
                    report.last_day_ipv4.u8(),
                    report.last_day_ipv4.u8(),
                )
                .await
                .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;
        }

        Ok(())
    }

    pub(crate) async fn check_if_historical_uptimes_exist_for_date(
        &self,
        date_iso_8601: &str,
    ) -> Result<bool, ValidatorApiStorageError> {
        self.manager
            .check_for_historical_uptime_existence(date_iso_8601)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }

    /// Removes all ipv4 and ipv6 statuses for all mixnodes and gateways that are older than the
    /// provided timestamp. This method is called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_statuses(
        &self,
        until: UnixTimestamp,
    ) -> Result<(), ValidatorApiStorageError> {
        self.manager
            .purge_old_mixnode_ipv4_statuses(until)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;
        self.manager
            .purge_old_mixnode_ipv6_statuses(until)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;
        self.manager
            .purge_old_gateway_ipv4_statuses(until)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;
        self.manager
            .purge_old_gateway_ipv6_statuses(until)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }

    ////////////////////////////////////////////////////////////////////////
    // TODO: Should all of the below really return a "NodeStatusApi" Errors?
    ////////////////////////////////////////////////////////////////////////

    /// Inserts information about starting new epoch rewarding into the database.
    /// Returns id of the newly created entry.
    ///
    /// # Arguments
    ///
    /// * `epoch_timestamp`: Unix timestamp of this rewarding epoch.
    pub(crate) async fn insert_started_epoch_rewarding(
        &self,
        epoch_timestamp: UnixTimestamp,
    ) -> Result<i64, ValidatorApiStorageError> {
        self.manager
            .insert_new_epoch_rewarding(epoch_timestamp)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }

    // /// Tries to obtain the most recent epoch rewarding entry currently stored.
    // ///
    // /// Returns None if no data exists.
    // pub(crate) async fn get_most_recent_epoch_rewarding_entry(
    //     &self,
    // ) -> Result<Option<EpochRewarding>, NodeStatusApiError> {
    //     self.manager
    //         .get_most_recent_epoch_rewarding_entry()
    //         .await
    //         .map_err(|_| NodeStatusApiError::InternalDatabaseError)
    // }

    /// Tries to obtain the epoch rewarding entry that has the provided timestamp.
    ///
    /// Returns None if no data exists.
    ///
    /// # Arguments
    ///
    /// * `epoch_timestamp`: Unix timestamp of this rewarding epoch.
    pub(super) async fn get_epoch_rewarding_entry(
        &self,
        epoch_timestamp: UnixTimestamp,
    ) -> Result<Option<EpochRewarding>, ValidatorApiStorageError> {
        self.manager
            .get_epoch_rewarding_entry(epoch_timestamp)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }

    /// Sets the `finished` field on the epoch rewarding to true and inserts the rewarding report into
    /// the database.
    ///
    /// # Arguments
    ///
    /// * `report`: report to insert into the database
    pub(crate) async fn finish_rewarding_epoch_and_insert_report(
        &self,
        report: RewardingReport,
    ) -> Result<(), ValidatorApiStorageError> {
        self.manager
            .update_finished_epoch_rewarding(report.epoch_rewarding_id)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        self.manager
            .insert_rewarding_report(report)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }

    /// Inserts new failed mixnode reward chunk information into the database.
    /// Returns id of the newly created entry.
    ///
    /// # Arguments
    ///
    /// * `failed_chunk`: chunk information to insert.
    pub(crate) async fn insert_failed_mixnode_reward_chunk(
        &self,
        failed_chunk: FailedMixnodeRewardChunk,
    ) -> Result<i64, ValidatorApiStorageError> {
        self.manager
            .insert_failed_mixnode_reward_chunk(failed_chunk)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }

    /// Inserts information into the database about a mixnode that might have been unfairly unrewarded this epoch.
    ///
    /// # Arguments
    ///
    /// * `mixnode`: mixnode information to insert.
    pub(crate) async fn insert_possibly_unrewarded_mixnode(
        &self,
        mixnode: PossiblyUnrewardedMixnode,
    ) -> Result<(), ValidatorApiStorageError> {
        self.manager
            .insert_possibly_unrewarded_mixnode(mixnode)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }
}
