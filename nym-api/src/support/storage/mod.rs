// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::test_route::TestRoute;
use crate::node_status_api::models::{
    GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport, MixnodeUptimeHistory,
    NymApiStorageError, Uptime,
};
use crate::node_status_api::{ONE_DAY, ONE_HOUR};
use crate::storage::manager::StorageManager;
use crate::storage::models::{NodeStatus, TestingRoute};
use crate::support::storage::models::{
    GatewayDetails, MixnodeDetails, TestedGatewayStatus, TestedMixnodeStatus,
};
use nym_mixnet_contract_common::MixId;
use nym_types::monitoring::{GatewayResult, MixnodeResult};
use rocket::fairing::AdHoc;
use sqlx::ConnectOptions;
use std::path::Path;
use time::OffsetDateTime;

use self::manager::{AvgGatewayReliability, AvgMixnodeReliability};

pub(crate) mod manager;
pub(crate) mod models;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct NymApiStorage {
    pub manager: StorageManager,
}

impl NymApiStorage {
    pub async fn init<P: AsRef<Path>>(database_path: P) -> Result<Self, NymApiStorageError> {
        // TODO: we can inject here more stuff based on our nym-api global config
        // struct. Maybe different pool size or timeout intervals?
        let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true);

        // TODO: do we want auto_vacuum ?

        opts.disable_statement_logging();

        let connection_pool = match sqlx::SqlitePool::connect_with(opts).await {
            Ok(db) => db,
            Err(err) => {
                error!("Failed to connect to SQLx database: {err}");
                return Err(err.into());
            }
        };

        if let Err(err) = sqlx::migrate!("./migrations").run(&connection_pool).await {
            error!("Failed to initialize SQLx database: {err}");
            return Err(err.into());
        }

        info!("Database migration finished!");

        let storage = NymApiStorage {
            manager: StorageManager { connection_pool },
        };

        Ok(storage)
    }

    #[deprecated(note = "TODO rocket: obsolete because it's used for Rocket")]
    pub(crate) fn stage(storage: NymApiStorage) -> AdHoc {
        AdHoc::try_on_ignite("SQLx Database", |rocket| async {
            Ok(rocket.manage(storage))
        })
    }

    #[allow(unused)]
    pub(crate) async fn mix_identity_to_mix_ids(
        &self,
        identity: &str,
    ) -> Result<Vec<MixId>, NymApiStorageError> {
        Ok(self
            .manager
            .get_mixnode_mix_ids_by_identity(identity)
            .await?)
    }

    #[allow(unused)]
    pub(crate) async fn mix_identity_to_latest_mix_id(
        &self,
        identity: &str,
    ) -> Result<Option<MixId>, NymApiStorageError> {
        Ok(self
            .mix_identity_to_mix_ids(identity)
            .await?
            .into_iter()
            .max())
    }

    pub(crate) async fn get_all_avg_gateway_reliability_in_last_24hr(
        &self,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgGatewayReliability>, NymApiStorageError> {
        let result = self
            .manager
            .get_all_avg_gateway_reliability_in_last_24hr(end_ts_secs)
            .await?;

        Ok(result)
    }

    pub(crate) async fn get_all_avg_mix_reliability_in_last_24hr(
        &self,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgMixnodeReliability>, NymApiStorageError> {
        let result = self
            .manager
            .get_all_avg_mix_reliability_in_last_24hr(end_ts_secs)
            .await?;

        Ok(result)
    }

    /// Gets all statuses for particular mixnode that were inserted
    /// since the provided timestamp.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode to query.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    async fn get_mixnode_statuses(
        &self,
        mix_id: MixId,
        since: i64,
    ) -> Result<Vec<NodeStatus>, NymApiStorageError> {
        let statuses = self
            .manager
            .get_mixnode_statuses_since(mix_id, since)
            .await?;

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
    ) -> Result<Vec<NodeStatus>, NymApiStorageError> {
        let statuses = self
            .manager
            .get_gateway_statuses_since(identity, since)
            .await?;

        Ok(statuses)
    }

    /// Tries to construct a status report for mixnode with the specified mix_id.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    pub(crate) async fn construct_mixnode_report(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeStatusReport, NymApiStorageError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        let statuses = self.get_mixnode_statuses(mix_id, day_ago).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if statuses.is_empty() {
            return Err(NymApiStorageError::MixnodeReportNotFound { mix_id });
        }

        // determine the number of runs the mixnode should have been online for
        let last_hour_runs_count = self
            .get_monitor_runs_count(hour_ago, now.unix_timestamp())
            .await?;
        let last_day_runs_count = self
            .get_monitor_runs_count(day_ago, now.unix_timestamp())
            .await?;

        let mixnode_owner =
            self.manager.get_mixnode_owner(mix_id).await?.expect(
                "The node doesn't have an owner even though we have status information on it!",
            );

        let mixnode_identity =
            self.manager.get_mixnode_identity_key(mix_id).await?.expect(
                "The node doesn't have an owner even though we have status information on it!",
            );

        Ok(MixnodeStatusReport::construct_from_last_day_reports(
            now,
            mix_id,
            mixnode_identity,
            mixnode_owner,
            statuses,
            last_hour_runs_count,
            last_day_runs_count,
        ))
    }

    pub(crate) async fn construct_gateway_report(
        &self,
        identity: &str,
    ) -> Result<GatewayStatusReport, NymApiStorageError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();

        let statuses = self.get_gateway_statuses(identity, day_ago).await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if statuses.is_empty() {
            return Err(NymApiStorageError::GatewayReportNotFound {
                identity: identity.to_owned(),
            });
        }

        // determine the number of runs the gateway should have been online for
        let last_hour_runs_count = self
            .get_monitor_runs_count(hour_ago, now.unix_timestamp())
            .await?;
        let last_day_runs_count = self
            .get_monitor_runs_count(day_ago, now.unix_timestamp())
            .await?;

        let gateway_owner = self.manager.get_gateway_owner(identity).await?.expect(
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
        mix_id: MixId,
    ) -> Result<MixnodeUptimeHistory, NymApiStorageError> {
        let history = self.manager.get_mixnode_historical_uptimes(mix_id).await?;

        if history.is_empty() {
            return Err(NymApiStorageError::MixnodeUptimeHistoryNotFound { mix_id });
        }

        let mixnode_owner =
            self.manager.get_mixnode_owner(mix_id).await?.expect(
                "The node doesn't have an owner even though we have uptime history for it!",
            );

        let mixnode_identity =
            self.manager.get_mixnode_identity_key(mix_id).await?.expect(
                "The node doesn't have an identity even though we have uptime history for it!",
            );

        Ok(MixnodeUptimeHistory::new(
            mix_id,
            mixnode_identity,
            mixnode_owner,
            history,
        ))
    }

    pub(crate) async fn get_gateway_uptime_history(
        &self,
        identity: &str,
    ) -> Result<GatewayUptimeHistory, NymApiStorageError> {
        let history = self
            .manager
            .get_gateway_historical_uptimes(identity)
            .await?;

        if history.is_empty() {
            return Err(NymApiStorageError::GatewayUptimeHistoryNotFound {
                identity: identity.to_owned(),
            });
        }

        let gateway_owner =
            self.manager.get_gateway_owner(identity).await?.expect(
                "The gateway doesn't have an owner even though we have uptime history for it!",
            );

        Ok(GatewayUptimeHistory::new(
            identity.to_owned(),
            gateway_owner,
            history,
        ))
    }

    pub(crate) async fn get_average_mixnode_uptime_in_the_last_24hrs(
        &self,
        mix_id: MixId,
        end_ts_secs: i64,
    ) -> Result<Uptime, NymApiStorageError> {
        let start = end_ts_secs - 86400;
        self.get_average_mixnode_uptime_in_time_interval(mix_id, start, end_ts_secs)
            .await
    }

    pub(crate) async fn get_average_gateway_uptime_in_the_last_24hrs(
        &self,
        identity: &str,
        end_ts_secs: i64,
    ) -> Result<Uptime, NymApiStorageError> {
        let start = end_ts_secs - 86400;
        self.get_average_gateway_uptime_in_time_interval(identity, start, end_ts_secs)
            .await
    }

    /// Based on the data available in the validator API, determines the average uptime of particular
    /// mixnode during the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_average_mixnode_uptime_in_time_interval(
        &self,
        mix_id: MixId,
        start: i64,
        end: i64,
    ) -> Result<Uptime, NymApiStorageError> {
        let mixnode_database_id = match self.manager.get_mixnode_database_id(mix_id).await? {
            Some(id) => id,
            None => return Ok(Uptime::zero()),
        };

        let reliability = self
            .manager
            .get_mixnode_average_reliability_in_interval(mixnode_database_id, start, end)
            .await?;

        if let Some(reliability) = reliability {
            Ok(Uptime::new(reliability))
        } else {
            Ok(Uptime::zero())
        }
    }

    /// Based on the data available in the validator API, determines the average uptime of particular
    /// gateway during the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `identity`: base58-encoded identity of the gateway.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    pub(crate) async fn get_average_gateway_uptime_in_time_interval(
        &self,
        identity: &str,
        start: i64,
        end: i64,
    ) -> Result<Uptime, NymApiStorageError> {
        let gateway_database_id = match self.manager.get_gateway_id(identity).await? {
            Some(id) => id,
            None => return Ok(Uptime::zero()),
        };

        let reliability = self
            .manager
            .get_gateway_average_reliability_in_interval(gateway_database_id, start, end)
            .await?;

        if let Some(reliability) = reliability {
            Ok(Uptime::new(reliability))
        } else {
            Ok(Uptime::zero())
        }
    }

    /// Obtain status reports of mixnodes that were active in the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `end`: unix timestamp indicating the upper bound interval of the selection.
    // NOTE: even though the arguments would suggest this function is generic in regards to
    // interval length, the constructed reports still assume the intervals are 24h in length.
    pub(crate) async fn get_all_active_mixnode_reports_in_interval(
        &self,
        start: i64,
        end: i64,
    ) -> Result<Vec<MixnodeStatusReport>, NymApiStorageError> {
        if (end - start) as u64 != ONE_DAY.as_secs() {
            warn!("Our current interval length breaks the 24h length assumption")
        }

        let hour_ago = end - ONE_HOUR.as_secs() as i64;

        // determine the number of runs the mixnodes should have been online for
        let last_hour_runs_count = self.get_monitor_runs_count(hour_ago, end).await?;
        let last_day_runs_count = self.get_monitor_runs_count(start, end).await?;

        let reports = self
            .manager
            .get_all_active_mixnodes_statuses_in_interval(start, end)
            .await?
            .into_iter()
            .map(|statuses| {
                MixnodeStatusReport::construct_from_last_day_reports(
                    OffsetDateTime::from_unix_timestamp(end).unwrap(),
                    statuses.mix_id,
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
    // interval length, the constructed reports still assume the intervals are 24h in length.
    pub(crate) async fn get_all_active_gateway_reports_in_interval(
        &self,
        start: i64,
        end: i64,
    ) -> Result<Vec<GatewayStatusReport>, NymApiStorageError> {
        if (end - start) as u64 != ONE_DAY.as_secs() {
            warn!("Our current interval length breaks the 24h length assumption")
        }

        let hour_ago = end - ONE_HOUR.as_secs() as i64;

        // determine the number of runs the mixnodes should have been online for
        let last_hour_runs_count = self.get_monitor_runs_count(hour_ago, end).await?;
        let last_day_runs_count = self.get_monitor_runs_count(start, end).await?;

        let reports = self
            .manager
            .get_all_active_gateways_statuses_in_interval(start, end)
            .await?
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
        monitor_run_db_id: i64,
        test_route: TestRoute,
    ) -> Result<(), NymApiStorageError> {
        // we MUST have those entries in the database, otherwise the route wouldn't have been chosen
        // in the first place
        let layer1_mix_db_id = self
            .manager
            .get_mixnode_database_id(test_route.layer_one_mix().mix_id)
            .await?
            .ok_or_else(|| NymApiStorageError::DatabaseInconsistency {
                reason: format!("could not get db id for layer1 mixnode from network monitor run {monitor_run_db_id}"),
            })?;

        let layer2_mix_db_id = self
            .manager
            .get_mixnode_database_id(test_route.layer_two_mix().mix_id)
            .await?
            .ok_or_else(|| NymApiStorageError::DatabaseInconsistency {
                reason: format!("could not get db id for layer2 mixnode from network monitor run {monitor_run_db_id}"),
            })?;

        let layer3_mix_db_id = self
            .manager
            .get_mixnode_database_id(test_route.layer_three_mix().mix_id)
            .await?
            .ok_or_else(|| NymApiStorageError::DatabaseInconsistency {
                reason: format!("could not get db id for layer3 mixnode from network monitor run {monitor_run_db_id}"),
            })?;

        let gateway_db_id = self
            .manager
            .get_gateway_id(&test_route.gateway().identity_key.to_base58_string())
            .await?
            .ok_or_else(|| NymApiStorageError::DatabaseInconsistency {
                reason: format!(
                    "could not get db id for gateway from network monitor run {monitor_run_db_id}"
                ),
            })?;

        self.manager
            .submit_testing_route_used(TestingRoute {
                gateway_db_id,
                layer1_mix_db_id,
                layer2_mix_db_id,
                layer3_mix_db_id,
                monitor_run_db_id,
            })
            .await?;
        Ok(())
    }

    /// Retrieves number of times particular mixnode was used as a core node during network monitor
    /// test runs since the specified unix timestamp. If no value is provided, last 30 days of data
    /// are used instead.
    ///
    /// # Arguments
    ///
    /// * `mix_id`: mix-id (as assigned by the smart contract) of the mixnode.
    /// * `since`: optional unix timestamp indicating the lower bound interval of the selection.
    pub(crate) async fn get_core_mixnode_status_count(
        &self,
        mix_id: MixId,
        since: Option<i64>,
    ) -> Result<i32, NymApiStorageError> {
        let db_id = self.manager.get_mixnode_database_id(mix_id).await?;

        if let Some(node_id) = db_id {
            let since = since
                .unwrap_or_else(|| (OffsetDateTime::now_utc() - (30 * ONE_DAY)).unix_timestamp());

            self.manager
                .get_mixnode_testing_route_presence_count_since(node_id, since)
                .await
                .map_err(|err| err.into())
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
    ) -> Result<i32, NymApiStorageError> {
        let node_id = self.manager.get_gateway_id(identity).await?;

        if let Some(node_id) = node_id {
            let since = since
                .unwrap_or_else(|| (OffsetDateTime::now_utc() - (30 * ONE_DAY)).unix_timestamp());

            self.manager
                .get_gateway_testing_route_presence_count_since(node_id, since)
                .await
                .map_err(|err| err.into())
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
        mixnode_results: Vec<MixnodeResult>,
        gateway_results: Vec<GatewayResult>,
        test_routes: Vec<TestRoute>,
    ) -> Result<(), NymApiStorageError> {
        info!("Submitting new node results to the database. There are {} mixnode results and {} gateway results", mixnode_results.len(), gateway_results.len());

        let now = OffsetDateTime::now_utc().unix_timestamp();

        let monitor_run_id = self.manager.insert_monitor_run(now).await?;

        self.manager
            .submit_mixnode_statuses(now, mixnode_results)
            .await?;

        self.manager
            .submit_gateway_statuses(now, gateway_results)
            .await?;

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
    ) -> Result<usize, NymApiStorageError> {
        let run_count = self.manager.get_monitor_runs_count(since, until).await?;

        if run_count < 0 {
            // I don't think it's ever possible for SQL to return a negative value from COUNT?
            return Err(NymApiStorageError::DatabaseInconsistency {
                reason: "Negative run count".to_string(),
            });
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
    ) -> Result<(), NymApiStorageError> {
        for report in mixnode_reports {
            // if this ever fails, we have a super weird error because we just constructed report for that node
            // and we never delete node data!
            let node_id = match self.manager.get_mixnode_database_id(report.mix_id).await? {
                Some(node_id) => node_id,
                None => {
                    error!(
                        "Somehow we failed to grab id of mixnode {} ({}) from the database!",
                        report.mix_id, report.identity
                    );
                    continue;
                }
            };

            self.manager
                .insert_mixnode_historical_uptime(node_id, today_iso_8601, report.last_day.u8())
                .await?;
        }

        for report in gateway_reports {
            // if this ever fails, we have a super weird error because we just constructed report for that node
            // and we never delete node data!
            let node_id = match self.manager.get_gateway_id(&report.identity).await? {
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
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn check_if_historical_uptimes_exist_for_date(
        &self,
        date_iso_8601: &str,
    ) -> Result<bool, NymApiStorageError> {
        self.manager
            .check_for_historical_uptime_existence(date_iso_8601)
            .await
            .map_err(|err| err.into())
    }

    /// Removes all ipv4 and ipv6 statuses for all mixnodes and gateways that are older than the
    /// provided timestamp. This method is called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_statuses(&self, until: i64) -> Result<(), NymApiStorageError> {
        self.manager.purge_old_mixnode_statuses(until).await?;
        self.manager
            .purge_old_gateway_statuses(until)
            .await
            .map_err(|err| err.into())
    }

    pub(crate) async fn get_mixnode_details_by_db_id(
        &self,
        id: i64,
    ) -> Result<Option<MixnodeDetails>, NymApiStorageError> {
        Ok(self.manager.get_mixnode_details_by_db_id(id).await?)
    }

    pub(crate) async fn get_gateway_details_by_db_id(
        &self,
        id: i64,
    ) -> Result<Option<GatewayDetails>, NymApiStorageError> {
        Ok(self.manager.get_gateway_details_by_db_id(id).await?)
    }

    pub(crate) async fn get_mixnode_detailed_statuses_count(
        &self,
        db_id: i64,
    ) -> Result<usize, NymApiStorageError> {
        Ok(self
            .manager
            .get_mixnode_statuses_count(db_id)
            .await?
            .try_into()
            .unwrap_or(usize::MAX))
    }

    pub(crate) async fn get_mixnode_detailed_statuses(
        &self,
        mix_id: MixId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<TestedMixnodeStatus>, NymApiStorageError> {
        Ok(self
            .manager
            .get_mixnode_statuses(mix_id, limit, offset)
            .await?)
    }

    pub(crate) async fn get_gateway_detailed_statuses_count(
        &self,
        db_id: i64,
    ) -> Result<usize, NymApiStorageError> {
        Ok(self
            .manager
            .get_gateway_statuses_count(db_id)
            .await?
            .try_into()
            .unwrap_or(usize::MAX))
    }

    pub(crate) async fn get_gateway_detailed_statuses(
        &self,
        gateway_identity: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<TestedGatewayStatus>, NymApiStorageError> {
        Ok(self
            .manager
            .get_gateway_statuses(gateway_identity, limit, offset)
            .await?)
    }
}
