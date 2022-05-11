// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::node_status_api::models::{HistoricalUptime, Uptime};
use crate::node_status_api::utils::ActiveNodeStatuses;
use crate::storage::models::{ActiveNode, NodeStatus, RewardingReport, TestingRoute};
use std::convert::TryFrom;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

pub struct AvgReliability {
    identity: String,
    value: Option<f32>,
}

impl AvgReliability {
    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn value(&self) -> f32 {
        self.value.unwrap_or_default()
    }
}

// all SQL goes here
impl StorageManager {
    pub(super) async fn get_all_avg_mix_reliability_in_last_24hr(
        &self,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgReliability>, sqlx::Error> {
        let start_ts_secs = end_ts_secs - 86400;
        self.get_all_avg_mix_reliability_in_interval(start_ts_secs, end_ts_secs)
            .await
    }

    pub(super) async fn get_all_avg_gateway_reliability_in_last_24hr(
        &self,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgReliability>, sqlx::Error> {
        let start_ts_secs = end_ts_secs - 86400;
        self.get_all_avg_gateway_reliability_in_interval(start_ts_secs, end_ts_secs)
            .await
    }

    pub(super) async fn get_all_avg_mix_reliability_in_interval(
        &self,
        start_ts_secs: i64,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgReliability>, sqlx::Error> {
        let result = sqlx::query_as!(
            AvgReliability,
            r#"
            SELECT
                d.identity as "identity: String",
                AVG(s.reliability) as "value: f32"
            FROM
                mixnode_details d
            JOIN
                mixnode_status s on d.id = s.mixnode_details_id
            WHERE
                timestamp >= ? AND
                timestamp <= ?
            GROUP BY 1
            "#,
            start_ts_secs,
            end_ts_secs
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(result)
    }

    pub(super) async fn get_all_avg_gateway_reliability_in_interval(
        &self,
        start_ts_secs: i64,
        end_ts_secs: i64,
    ) -> Result<Vec<AvgReliability>, sqlx::Error> {
        let result = sqlx::query_as!(
            AvgReliability,
            r#"
            SELECT
                d.identity as "identity: String",
                AVG(reliability) as "value: f32"
            FROM
                gateway_details d
            JOIN
                gateway_status s on d.id = s.gateway_details_id
            WHERE
                timestamp >= ? AND
                timestamp <= ?
            GROUP BY 1
            "#,
            start_ts_secs,
            end_ts_secs
        )
        .fetch_all(&self.connection_pool)
        .await?;
        Ok(result)
    }

    /// Tries to obtain row id of given mixnode given its identity.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the mixnode.
    pub(super) async fn get_mixnode_id(&self, identity: &str) -> Result<Option<i64>, sqlx::Error> {
        let id = sqlx::query!(
            "SELECT id FROM mixnode_details WHERE identity = ?",
            identity
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.id);

        Ok(id)
    }

    /// Tries to obtain row id of given gateway given its identity
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the gateway.
    pub(super) async fn get_gateway_id(&self, identity: &str) -> Result<Option<i64>, sqlx::Error> {
        let id = sqlx::query!(
            "SELECT id FROM gateway_details WHERE identity = ?",
            identity
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.id);

        Ok(id)
    }

    /// Tries to obtain owner value of given mixnode given its identity
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the mixnode.
    pub(super) async fn get_mixnode_owner(
        &self,
        identity: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let owner = sqlx::query!(
            "SELECT owner FROM mixnode_details WHERE identity = ?",
            identity
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.owner);

        Ok(owner)
    }

    /// Tries to obtain owner value of given gateway given its identity
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the gateway.
    pub(super) async fn get_gateway_owner(
        &self,
        identity: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let owner = sqlx::query!(
            "SELECT owner FROM gateway_details WHERE identity = ?",
            identity
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.owner);

        Ok(owner)
    }

    /// Gets all reliability statuses for mixnode with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the mixnode.
    /// * `timestamp`: unix timestamp of the lower bound of the selection.
    pub(super) async fn get_mixnode_statuses_since(
        &self,
        identity: &str,
        timestamp: i64,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, reliability as "reliability: u8"
                    FROM mixnode_status
                    JOIN mixnode_details
                    ON mixnode_status.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.identity=? AND mixnode_status.timestamp > ?;
            "#,
            identity,
            timestamp,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets all reliability statuses for gateway with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the gateway.
    /// * `timestamp`: unix timestamp of the lower bound of the selection.
    pub(super) async fn get_gateway_statuses_since(
        &self,
        identity: &str,
        timestamp: i64,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, reliability as "reliability: u8"
                    FROM gateway_status
                    JOIN gateway_details
                    ON gateway_status.gateway_details_id = gateway_details.id
                    WHERE gateway_details.identity=? AND gateway_status.timestamp > ?;
            "#,
            identity,
            timestamp,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets the historical daily uptime associated with the particular mixnode
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the mixnode.
    pub(super) async fn get_mixnode_historical_uptimes(
        &self,
        identity: &str,
    ) -> Result<Vec<HistoricalUptime>, sqlx::Error> {
        let uptimes = sqlx::query!(
            r#"
                SELECT date, uptime
                    FROM mixnode_historical_uptime
                    JOIN mixnode_details
                    ON mixnode_historical_uptime.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.identity = ?
                    ORDER BY date ASC
            "#,
            identity
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        // filter out nodes with valid uptime (in theory all should be 100% valid since we insert them ourselves, but
        // better safe than sorry and not use an unwrap)
        .filter_map(|row| {
            Uptime::try_from(row.uptime.unwrap_or_default())
                .map(|uptime| HistoricalUptime {
                    date: row.date.unwrap_or_default(),
                    uptime,
                })
                .ok()
        })
        .collect();

        Ok(uptimes)
    }

    /// Gets the historical daily uptime associated with the particular gateway
    ///
    /// # Arguments
    ///
    /// * `identity`: identity (base58-encoded public key) of the gateway.
    pub(super) async fn get_gateway_historical_uptimes(
        &self,
        identity: &str,
    ) -> Result<Vec<HistoricalUptime>, sqlx::Error> {
        let uptimes = sqlx::query!(
            r#"
                SELECT date, uptime
                    FROM gateway_historical_uptime
                    JOIN gateway_details
                    ON gateway_historical_uptime.gateway_details_id = gateway_details.id
                    WHERE gateway_details.identity = ?
                    ORDER BY date ASC
            "#,
            identity
        )
        .fetch_all(&self.connection_pool)
        .await?
        .into_iter()
        // filter out nodes with valid uptime (in theory all should be 100% valid since we insert them ourselves, but
        // better safe than sorry and not use an unwrap)
        .filter_map(|row| {
            Uptime::try_from(row.uptime.unwrap_or_default())
                .map(|uptime| HistoricalUptime {
                    date: row.date.unwrap_or_default(),
                    uptime,
                })
                .ok()
        })
        .collect();

        Ok(uptimes)
    }

    /// Gets all reliability statuses for mixnode with particular id that were inserted
    /// into the database within the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(super) async fn get_mixnode_statuses_by_id(
        &self,
        id: i64,
        since: i64,
        until: i64,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, reliability as "reliability: u8"
                    FROM mixnode_status
                    WHERE mixnode_details_id=? AND timestamp > ? AND timestamp < ?;
            "#,
            id,
            since,
            until,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    pub(super) async fn get_average_reliability_in_interval(
        &self,
        id: i64,
        start: i64,
        end: i64,
    ) -> Result<Option<f32>, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT AVG(reliability) as "reliability: f32" FROM mixnode_status
            WHERE mixnode_details_id= ? AND timestamp >= ? AND timestamp <= ?
            "#,
            id,
            start,
            end
        )
        .fetch_one(&self.connection_pool)
        .await?;
        Ok(result.reliability)
    }

    /// Gets all reliability statuses for gateway with particular id that were inserted
    /// into the database within the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(super) async fn get_gateway_statuses_by_id(
        &self,
        id: i64,
        since: i64,
        until: i64,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, reliability as "reliability: u8"
                    FROM gateway_status
                    WHERE gateway_details_id=? AND timestamp > ? AND timestamp < ?;
            "#,
            id,
            since,
            until,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Tries to submit mixnode [`NodeResult`] from the network monitor to the database.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: unix timestamp indicating when the measurements took place.
    /// * `mixnode_results`: reliability results of each node that got tested.
    pub(super) async fn submit_mixnode_statuses(
        &self,
        timestamp: i64,
        mixnode_results: Vec<NodeResult>,
    ) -> Result<(), sqlx::Error> {
        // insert it all in a transaction to make sure all nodes are updated at the same time
        // (plus it's a nice guard against new nodes)
        let mut tx = self.connection_pool.begin().await?;
        for mixnode_result in mixnode_results {
            let mixnode_id = sqlx::query!(
                r#"
                    INSERT OR IGNORE INTO mixnode_details(identity, owner) VALUES (?, ?);
                    SELECT id FROM mixnode_details WHERE identity = ?;
                "#,
                mixnode_result.identity,
                mixnode_result.owner,
                mixnode_result.identity,
            )
            .fetch_one(&mut tx)
            .await?
            .id;

            // insert the actual status
            sqlx::query!(
                    r#"
                        INSERT INTO mixnode_status (mixnode_details_id, reliability, timestamp) VALUES (?, ?, ?);
                    "#,
                    mixnode_id,
                    mixnode_result.reliability,
                    timestamp
                )
                .execute(&mut tx)
                .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }

    /// Tries to submit gateway [`NodeResult`] from the network monitor to the database.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: unix timestamp indicating when the measurements took place.
    /// * `gateway_results`: reliability results of each node that got tested.
    pub(super) async fn submit_gateway_statuses(
        &self,
        timestamp: i64,
        gateway_results: Vec<NodeResult>,
    ) -> Result<(), sqlx::Error> {
        // insert it all in a transaction to make sure all nodes are updated at the same time
        // (plus it's a nice guard against new nodes)
        let mut tx = self.connection_pool.begin().await?;

        for gateway_result in gateway_results {
            // if gateway info doesn't exist, insert it and get its id

            // same ID "problem" as described for mixnode insertion
            let gateway_id = sqlx::query!(
                r#"
                    INSERT OR IGNORE INTO gateway_details(identity, owner) VALUES (?, ?);
                    SELECT id FROM gateway_details WHERE identity = ?;
                "#,
                gateway_result.identity,
                gateway_result.owner,
                gateway_result.identity,
            )
            .fetch_one(&mut tx)
            .await?
            .id;

            // insert the actual status
            sqlx::query!(
                    r#"
                        INSERT INTO gateway_status (gateway_details_id, reliability, timestamp) VALUES (?, ?, ?);
                    "#,
                    gateway_id,
                    gateway_result.reliability,
                    timestamp
                )
                .execute(&mut tx)
                .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }

    /// Saves the information about which nodes were used as core nodes during this particular
    /// network monitor test run.
    ///
    /// # Arguments
    ///
    /// * `testing_route`: test route used for this particular network monitor run.
    pub(super) async fn submit_testing_route_used(
        &self,
        testing_route: TestingRoute,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO testing_route 
                (gateway_id, layer1_mix_id, layer2_mix_id, layer3_mix_id, monitor_run_id) 
                VALUES (?, ?, ?, ?, ?);
            "#,
            testing_route.gateway_id,
            testing_route.layer1_mix_id,
            testing_route.layer2_mix_id,
            testing_route.layer3_mix_id,
            testing_route.monitor_run_id,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Get the number of times mixnode with the particular id is present in any `testing_route`
    /// since the provided unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `mixnode_id`: id (as saved in the database) of the mixnode.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    pub(super) async fn get_mixnode_testing_route_presence_count_since(
        &self,
        mixnode_id: i64,
        since: i64,
    ) -> Result<i32, sqlx::Error> {
        let count = sqlx::query!(
            r#"
                SELECT COUNT(*) as count FROM
                (
                    SELECT monitor_run_id 
                    FROM testing_route 
                    WHERE testing_route.layer1_mix_id = ? OR testing_route.layer2_mix_id = ? OR testing_route.layer3_mix_id = ?
                ) testing_route
                JOIN 
                (
                    SELECT id 
                    FROM monitor_run 
                    WHERE monitor_run.timestamp > ?
                ) monitor_run
                ON monitor_run.id = testing_route.monitor_run_id;
            "#,
            mixnode_id,
            mixnode_id,
            mixnode_id,
            since,
        ).fetch_one(&self.connection_pool)
            .await?
            .count;

        Ok(count)
    }

    /// Get the number of times gateway with the particular id is present in any `testing_route`
    /// since the provided unix timestamp.
    ///
    /// # Arguments
    ///
    /// * `gateway_id`: id (as saved in the database) of the gateway.
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    pub(super) async fn get_gateway_testing_route_presence_count_since(
        &self,
        gateway_id: i64,
        since: i64,
    ) -> Result<i32, sqlx::Error> {
        let count = sqlx::query!(
            r#"
                SELECT COUNT(*) as count FROM
                (
                    SELECT monitor_run_id 
                    FROM testing_route 
                    WHERE testing_route.gateway_id = ?
                ) testing_route
                JOIN 
                (
                    SELECT id 
                    FROM monitor_run 
                    WHERE monitor_run.timestamp > ?
                ) monitor_run
                ON monitor_run.id = testing_route.monitor_run_id;
            "#,
            gateway_id,
            since,
        )
        .fetch_one(&self.connection_pool)
        .await?
        .count;

        Ok(count)
    }

    /// Checks whether there are already any historical uptimes with this particular date.
    pub(super) async fn check_for_historical_uptime_existence(
        &self,
        today_iso_8601: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query!(
            "SELECT EXISTS (SELECT 1 FROM mixnode_historical_uptime WHERE date = ?) AS 'exists'",
            today_iso_8601
        )
        .fetch_one(&self.connection_pool)
        .await
        .map(|result| result.exists == 1)
    }

    /// Creates new entry for mixnode historical uptime
    ///
    /// # Arguments
    ///
    /// * `node_id`: id of the mixnode (as inserted in `mixnode_details_id` table).
    /// * `date`: date associated with the uptime represented in ISO 8601, i.e. YYYY-MM-DD.
    /// * `uptime`: the actual uptime of the node during the specified day.
    pub(super) async fn insert_mixnode_historical_uptime(
        &self,
        node_id: i64,
        date: &str,
        uptime: u8,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO mixnode_historical_uptime(mixnode_details_id, date, uptime) VALUES (?, ?, ?)",
                node_id,
                date,
                uptime,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }

    /// Creates new entry for gateway historical uptime
    ///
    /// # Arguments
    ///
    /// * `node_id`: id of the gateway (as inserted in `gateway_details_id` table).
    /// * `date`: date associated with the uptime represented in ISO 8601, i.e. YYYY-MM-DD.
    /// * `uptime`: the actual uptime of the node during the specified day.
    pub(super) async fn insert_gateway_historical_uptime(
        &self,
        node_id: i64,
        date: &str,
        uptime: u8,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO gateway_historical_uptime(gateway_details_id, date, uptime) VALUES (?, ?, ?)",
                node_id,
                date,
                uptime,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }

    /// Creates a database entry for a finished network monitor test run.
    /// Returns id of the newly created entry.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: unix timestamp at which the monitor test run has occurred
    pub(super) async fn insert_monitor_run(&self, timestamp: i64) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!("INSERT INTO monitor_run(timestamp) VALUES (?)", timestamp)
            .execute(&self.connection_pool)
            .await?;
        Ok(res.last_insert_rowid())
    }

    /// Obtains number of network monitor test runs that have occurred within the specified interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(super) async fn get_monitor_runs_count(
        &self,
        since: i64,
        until: i64,
    ) -> Result<i32, sqlx::Error> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM monitor_run WHERE timestamp > ? AND timestamp < ?",
            since,
            until,
        )
        .fetch_one(&self.connection_pool)
        .await?
        .count;
        Ok(count)
    }

    /// Removes all statuses for all mixnodes that are older than the
    /// provided timestamp. This method is indirectly called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(super) async fn purge_old_mixnode_statuses(
        &self,
        timestamp: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM mixnode_status WHERE timestamp < ?", timestamp)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    /// Removes all statuses for all gateways that are older than the
    /// provided timestamp. This method is indirectly called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(super) async fn purge_old_gateway_statuses(
        &self,
        timestamp: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM gateway_status WHERE timestamp < ?", timestamp)
            .execute(&self.connection_pool)
            .await?;
        Ok(())
    }

    /// Returns public key, owner and id of all mixnodes that have had any statuses submitted
    /// within the provided time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: indicates the lower bound timestamp for deciding whether given mixnode is active
    /// * `until`: indicates the upper bound timestamp for deciding whether given mixnode is active
    pub(super) async fn get_all_active_mixnodes_in_interval(
        &self,
        since: i64,
        until: i64,
    ) -> Result<Vec<ActiveNode>, sqlx::Error> {
        // find mixnode details of all nodes that have had at least 1 status information since the provided
        // timestamp
        // TODO: I dont know if theres a potential issue of if we have a lot of inactive nodes that
        // haven't mixed in ages, they might increase the query times?
        sqlx::query_as!(
            ActiveNode,
            r#"
                SELECT DISTINCT identity, owner, id
                    FROM mixnode_details
                    JOIN mixnode_status
                    ON mixnode_details.id = mixnode_status.mixnode_details_id
                    WHERE EXISTS (
                        SELECT 1 FROM mixnode_status WHERE timestamp > ? AND timestamp < ?
                    )
            "#,
            since,
            until
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Returns public key, owner and id of all gateways that have had any statuses submitted
    /// within the provided time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: indicates the lower bound timestamp for deciding whether given gateway is active
    /// * `until`: indicates the upper bound timestamp for deciding whether given gateway is active
    pub(super) async fn get_all_active_gateways_in_interval(
        &self,
        since: i64,
        until: i64,
    ) -> Result<Vec<ActiveNode>, sqlx::Error> {
        sqlx::query_as!(
            ActiveNode,
            r#"
                SELECT DISTINCT identity, owner, id
                    FROM gateway_details
                    JOIN gateway_status
                    ON gateway_details.id = gateway_status.gateway_details_id
                    WHERE EXISTS (
                        SELECT 1 FROM gateway_status WHERE timestamp > ? AND timestamp < ?
                    )
            "#,
            since,
            until,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    // /// Tries to obtain the most recent interval rewarding entry currently stored.
    // ///
    // /// Returns None if no data exists.
    // pub(super) async fn get_most_recent_interval_rewarding_entry(
    //     &self,
    // ) -> Result<Option<IntervalRewarding>, sqlx::Error> {
    //     sqlx::query_as!(
    //         IntervalRewarding,
    //         r#"
    //             SELECT * FROM interval_rewarding
    //             ORDER BY interval_timestamp DESC
    //             LIMIT 1
    //         "#,
    //     )
    //     .fetch_optional(&self.connection_pool)
    //     .await
    // }

    /// Inserts new rewarding report into the database.
    ///
    /// # Arguments
    ///
    /// * `report`: report to insert into the database
    pub(super) async fn insert_rewarding_report(
        &self,
        report: RewardingReport,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO rewarding_report
                (interval_rewarding_id, eligible_mixnodes, possibly_unrewarded_mixnodes)
                VALUES (?, ?, ?);
            "#,
            report.interval_rewarding_id,
            report.eligible_mixnodes,
            report.possibly_unrewarded_mixnodes,
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Obtains all statuses of active mixnodes from the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(super) async fn get_all_active_mixnodes_statuses_in_interval(
        &self,
        since: i64,
        until: i64,
    ) -> Result<Vec<ActiveNodeStatuses>, sqlx::Error> {
        let active_nodes = self
            .get_all_active_mixnodes_in_interval(since, until)
            .await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for active_node in active_nodes.into_iter() {
            let statuses = self
                .get_mixnode_statuses_by_id(active_node.id, since, until)
                .await?;

            let statuses = ActiveNodeStatuses {
                identity: active_node.identity,
                owner: active_node.owner,
                statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }

    /// Obtains all statuses of active gateways from the specified time interval.
    ///
    /// # Arguments
    ///
    /// * `since`: unix timestamp indicating the lower bound interval of the selection.
    /// * `until`: unix timestamp indicating the upper bound interval of the selection.
    pub(super) async fn get_all_active_gateways_statuses_in_interval(
        &self,
        since: i64,
        until: i64,
    ) -> Result<Vec<ActiveNodeStatuses>, sqlx::Error> {
        let active_nodes = self
            .get_all_active_gateways_in_interval(since, until)
            .await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for active_node in active_nodes.into_iter() {
            let statuses = self
                .get_gateway_statuses_by_id(active_node.id, since, until)
                .await?;

            let statuses = ActiveNodeStatuses {
                identity: active_node.identity,
                owner: active_node.owner,
                statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }

    /// Creates new encrypted blinded signature response entry for a given deposit tx hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash`: hash of the deposit transaction.
    /// * `blinded_signature_response`: the encrypted blinded signature response.
    #[cfg(feature = "coconut")]
    pub(super) async fn insert_blinded_signature_response(
        &self,
        tx_hash: &str,
        blinded_signature_response: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO signed_deposit(tx_hash, blinded_signature_response) VALUES (?, ?)",
            tx_hash,
            blinded_signature_response
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Tries to obtain encrypted blinded signature response for a given transaction hash.
    ///
    /// # Arguments
    ///
    /// * `tx_hash`: transaction hash of the deposit.
    #[cfg(feature = "coconut")]
    pub(super) async fn get_blinded_signature_response(
        &self,
        tx_hash: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let blinded_signature_response = sqlx::query!(
            "SELECT blinded_signature_response FROM signed_deposit WHERE tx_hash = ?",
            tx_hash
        )
        .fetch_optional(&self.connection_pool)
        .await?
        .map(|row| row.blinded_signature_response);

        Ok(blinded_signature_response)
    }
}
