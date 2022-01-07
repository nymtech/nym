// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::network_monitor::test_route::TestRoute;
use crate::node_status_api::models::{
    GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport, MixnodeUptimeHistory, Uptime,
    ValidatorApiStorageError,
};
use crate::node_status_api::{ONE_DAY, ONE_HOUR};
use crate::storage::manager::StorageManager;
use crate::storage::models::{
    EpochRewarding, FailedMixnodeRewardChunk, NodeStatus, PossiblyUnrewardedMixnode,
    RewardingReport, TestingRoute,
};
use rocket::fairing::{self, AdHoc};
use rocket::{Build, Rocket};
use sqlx::ConnectOptions;
use std::path::PathBuf;
use time::OffsetDateTime;

pub(crate) mod manager;
pub(crate) mod models;

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

    /// Gets all statuses for particular mixnode that were inserted
    /// since the provided timestamp.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity key of the mixnode to query.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    async fn get_mixnode_statuses(
        &self,
        identity: &str,
        since: i64,
    ) -> Result<Vec<NodeStatus>, ValidatorApiStorageError> {
        let statuses = self
            .manager
            .get_mixnode_statuses_since(identity, since)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        Ok(statuses)
    }

    /// Gets all statuses for particular gateway that were inserted
    /// since the provided timestamp.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity key of the gateway to query.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    async fn get_gateway_statuses(
        &self,
        identity: &str,
        since: i64,
    ) -> Result<Vec<NodeStatus>, ValidatorApiStorageError> {
        let statuses = self
            .manager
            .get_gateway_statuses_since(identity, since)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        Ok(statuses)
    }

    /// Tries to construct a status report for mixnode with the specified identity.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the mixnode.
    pub(crate) async fn construct_mixnode_report(
        &self,
        identity: &str,
    ) -> Result<MixnodeStatusReport, ValidatorApiStorageError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        let statuses = self.get_mixnode_statuses(identity, day_ago).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if statuses.is_empty() {
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
            statuses,
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

        let statuses = self.get_gateway_statuses(identity, day_ago).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if statuses.is_empty() {
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
            statuses,
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

    /// Based on the data available in the validator API, determines the average uptime of particular
    /// mixnode during the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `identity`: base58-encoded identity of the mixnode.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_average_mixnode_uptime_in_interval(
        &self,
        identity: &str,
        start: i64,
        end: i64,
    ) -> Result<Uptime, ValidatorApiStorageError> {
        let mixnode_database_id = match self
            .manager
            .get_mixnode_id(identity)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
        {
            Some(id) => id,
            None => return Ok(Uptime::zero()),
        };

        let monitor_runs = self.get_monitor_runs_count(start, end).await?;
        let mixnode_statuses = self
            .manager
            .get_mixnode_statuses_by_id(mixnode_database_id, start, end)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        let mut total: f32 = 0.0;
        for mixnode_status in mixnode_statuses {
            total += mixnode_status.reliability as f32;
        }

        let uptime = match Uptime::from_uptime_sum(total, monitor_runs) {
            Ok(uptime) => uptime,
            Err(_) => {
                // this should really ever happen...
                error!("mixnode {} has uptime > 100!", identity);
                Uptime::default()
            }
        };

        Ok(uptime)
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
        start: i64,
        end: i64,
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
                    statuses.statuses,
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
        start: i64,
        end: i64,
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
                    statuses.statuses,
                    last_hour_runs_count,
                    last_day_runs_count,
                )
            })
            .collect();

        Ok(reports)
    }

    /// Saves information about test route used during the network monitor run to the database.
    ///
    /// # Arguments
    ///
    /// * `monitor_run_id` id (as saved in the database) of the associated network monitor test run.
    /// * `test_route`: one of the test routes used during network testing.
    async fn insert_test_route(
        &self,
        monitor_run_id: i64,
        test_route: TestRoute,
    ) -> Result<(), ValidatorApiStorageError> {
        // we MUST have those entries in the database, otherwise the route wouldn't have been chosen
        // in the first place
        let layer1_mix_id = self
            .manager
            .get_mixnode_id(&test_route.layer_one_mix().identity_key.to_base58_string())
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            .ok_or(ValidatorApiStorageError::InternalDatabaseError)?;

        let layer2_mix_id = self
            .manager
            .get_mixnode_id(&test_route.layer_two_mix().identity_key.to_base58_string())
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            .ok_or(ValidatorApiStorageError::InternalDatabaseError)?;

        let layer3_mix_id = self
            .manager
            .get_mixnode_id(&test_route.layer_three_mix().identity_key.to_base58_string())
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            .ok_or(ValidatorApiStorageError::InternalDatabaseError)?;

        let gateway_id = self
            .manager
            .get_gateway_id(&test_route.gateway().identity_key.to_base58_string())
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?
            .ok_or(ValidatorApiStorageError::InternalDatabaseError)?;

        self.manager
            .submit_testing_route_used(TestingRoute {
                gateway_id,
                layer1_mix_id,
                layer2_mix_id,
                layer3_mix_id,
                monitor_run_id,
            })
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;
        Ok(())
    }

    /// Retrieves number of times particular mixnode was used as a core node during network monitor
    /// test runs since the specified unix timestamp. If no value is provided, last 30 days of data
    /// are used instead.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the mixnode.
    /// * `since`: optional unix timestamp indicating the lower bound interval of the selection.
    pub(crate) async fn get_core_mixnode_status_count(
        &self,
        identity: &str,
        since: Option<i64>,
    ) -> Result<i32, ValidatorApiStorageError> {
        let node_id = self
            .manager
            .get_mixnode_id(identity)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        if let Some(node_id) = node_id {
            let since = since
                .unwrap_or_else(|| (OffsetDateTime::now_utc() - (30 * ONE_DAY)).unix_timestamp());

            self.manager
                .get_mixnode_testing_route_presence_count_since(node_id, since)
                .await
                .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
        } else {
            Ok(0)
        }
    }

    /// Retrieves number of times particular gateway was used as a core node during network monitor
    /// test runs since the specified unix timestamp. If no value is provided, last 30 days of data
    /// are used instead.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the gateway.
    /// * `since`: optional unix timestamp indicating the lower bound interval of the selection.
    pub(crate) async fn get_core_gateway_status_count(
        &self,
        identity: &str,
        since: Option<i64>,
    ) -> Result<i32, ValidatorApiStorageError> {
        let node_id = self
            .manager
            .get_gateway_id(identity)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        if let Some(node_id) = node_id {
            let since = since
                .unwrap_or_else(|| (OffsetDateTime::now_utc() - (30 * ONE_DAY)).unix_timestamp());

            self.manager
                .get_gateway_testing_route_presence_count_since(node_id, since)
                .await
                .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
        } else {
            Ok(0)
        }
    }

    /// Inserts an entry to the database with the network monitor test run information
    /// that has occurred at this instant alongside the results of all the measurements performed.
    ///
    /// # Arguments
    ///
    /// * `mixnode_results`:
    /// * `gateway_results`:
    /// * `route_results`:
    pub(crate) async fn insert_monitor_run_results(
        &self,
        mixnode_results: Vec<NodeResult>,
        gateway_results: Vec<NodeResult>,
        test_routes: Vec<TestRoute>,
    ) -> Result<(), ValidatorApiStorageError> {
        info!("Submitting new node results to the database. There are {} mixnode results and {} gateway results", mixnode_results.len(), gateway_results.len());

        let now = OffsetDateTime::now_utc().unix_timestamp();

        let monitor_run_id = self
            .manager
            .insert_monitor_run(now)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        self.manager
            .submit_mixnode_statuses(now, mixnode_results)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        self.manager
            .submit_gateway_statuses(now, gateway_results)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;

        for test_route in test_routes {
            self.insert_test_route(monitor_run_id, test_route).await?;
        }

        Ok(())
    }

    /// Obtains number of network monitor test runs that have occurred within the specified interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_monitor_runs_count(
        &self,
        since: i64,
        until: i64,
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
                .insert_mixnode_historical_uptime(node_id, today_iso_8601, report.last_day.u8())
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
                .insert_gateway_historical_uptime(node_id, today_iso_8601, report.last_day.u8())
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
        until: i64,
    ) -> Result<(), ValidatorApiStorageError> {
        self.manager
            .purge_old_mixnode_statuses(until)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)?;
        self.manager
            .purge_old_gateway_statuses(until)
            .await
            .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
    }

    ////////////////////////////////////////////////////////////////////////
    // TODO: Should all of the below really return a "ValidatorApiStorageError" Errors?
    ////////////////////////////////////////////////////////////////////////

    /// Inserts information about starting new epoch rewarding into the database.
    /// Returns id of the newly created entry.
    ///
    /// # Arguments
    ///
    /// * `epoch_timestamp`: Unix timestamp of this rewarding epoch.
    pub(crate) async fn insert_started_epoch_rewarding(
        &self,
        epoch_timestamp: i64,
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
    // ) -> Result<Option<EpochRewarding>, ValidatorApiStorageError> {
    //     self.manager
    //         .get_most_recent_epoch_rewarding_entry()
    //         .await
    //         .map_err(|_| ValidatorApiStorageError::InternalDatabaseError)
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
        epoch_timestamp: i64,
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
