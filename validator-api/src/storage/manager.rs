// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::node_status_api::models::{HistoricalUptime, Uptime};
use crate::node_status_api::utils::ActiveNodeDayStatuses;
use crate::storage::models::{
    ActiveNode, FailedGatewayRewardChunk, FailedMixnodeRewardChunk, NodeStatus,
    PossiblyUnrewardedGateway, PossiblyUnrewardedMixnode, RewardingReport,
};
use crate::storage::UnixTimestamp;
use std::convert::TryFrom;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(super) connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
    /// Tries to obtain row id of given mixnode given its identity
    pub(crate) async fn get_mixnode_id(&self, identity: &str) -> Result<Option<i64>, sqlx::Error> {
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
    pub(crate) async fn get_gateway_id(&self, identity: &str) -> Result<Option<i64>, sqlx::Error> {
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
    pub(crate) async fn get_mixnode_owner(
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
    pub(crate) async fn get_gateway_owner(
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

    /// Gets all ipv4 statuses for mixnode with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    pub(crate) async fn get_mixnode_ipv4_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv4_status
                    JOIN mixnode_details
                    ON mixnode_ipv4_status.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.identity=? AND mixnode_ipv4_status.timestamp > ?;
            "#,
            identity,
            timestamp,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets all ipv6 statuses for mixnode with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    pub(crate) async fn get_mixnode_ipv6_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv6_status
                    JOIN mixnode_details
                    ON mixnode_ipv6_status.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.identity=? AND mixnode_ipv6_status.timestamp > ?;
            "#,
            identity,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets all ipv4 statuses for gateway with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    pub(crate) async fn get_gateway_ipv4_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv4_status
                    JOIN gateway_details
                    ON gateway_ipv4_status.gateway_details_id = gateway_details.id
                    WHERE gateway_details.identity=? AND gateway_ipv4_status.timestamp > ?;
            "#,
            identity,
            timestamp,
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets all ipv6 statuses for gateway with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    pub(crate) async fn get_gateway_ipv6_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv6_status
                    JOIN gateway_details
                    ON gateway_ipv6_status.gateway_details_id = gateway_details.id
                    WHERE gateway_details.identity=? AND gateway_ipv6_status.timestamp > ?;
            "#,
            identity,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets the historical daily uptime associated with the particular mixnode
    pub(crate) async fn get_mixnode_historical_uptimes(
        &self,
        identity: &str,
    ) -> Result<Vec<HistoricalUptime>, sqlx::Error> {
        let uptimes = sqlx::query!(
            r#"
                SELECT date, ipv4_uptime, ipv6_uptime
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
    pub(crate) async fn get_gateway_historical_uptimes(
        &self,
        identity: &str,
    ) -> Result<Vec<HistoricalUptime>, sqlx::Error> {
        let uptimes = sqlx::query!(
            r#"
                SELECT date, ipv4_uptime, ipv6_uptime
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

    /// Gets all ipv4 statuses for mixnode with particular id that were inserted
    /// into the database after the specified unix timestamp.
    pub(crate) async fn get_mixnode_ipv4_statuses_since_by_id(
        &self,
        id: i64,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv4_status
                    WHERE mixnode_details_id=? AND timestamp > ?;
            "#,
            id,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets all ipv6 statuses for mixnode with particular id that were inserted
    /// into the database after the specified unix timestamp.
    pub(crate) async fn get_mixnode_ipv6_statuses_since_by_id(
        &self,
        id: i64,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv6_status
                    WHERE mixnode_details_id=? AND timestamp > ?;
            "#,
            id,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets all ipv4 statuses for gateway with particular id that were inserted
    /// into the database after the specified unix timestamp.
    pub(crate) async fn get_gateway_ipv4_statuses_since_by_id(
        &self,
        id: i64,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv4_status
                    WHERE gateway_details_id=? AND timestamp > ?;
            "#,
            id,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Gets all ipv6 statuses for gateway with particular id that were inserted
    /// into the database after the specified unix timestamp.
    pub(crate) async fn get_gateway_ipv6_statuses_since_by_id(
        &self,
        id: i64,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        sqlx::query_as!(
            NodeStatus,
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv6_status
                    WHERE gateway_details_id=? AND timestamp > ?;
            "#,
            id,
            timestamp
        )
        .fetch_all(&self.connection_pool)
        .await
    }

    /// Tries to submit mixnode [`NodeResult`] from the network monitor to the database.
    pub(crate) async fn submit_mixnode_statuses(
        &self,
        timestamp: UnixTimestamp,
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

            // insert ipv4 status
            sqlx::query!(
                r#"
                    INSERT INTO mixnode_ipv4_status (mixnode_details_id, up, timestamp) VALUES (?, ?, ?);
                "#,
                mixnode_id,
                mixnode_result.working_ipv4,
                timestamp
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
                timestamp
            )
                .execute(&mut tx)
                .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }

    /// Tries to submit gateway [`NodeResult`] from the network monitor to the database.
    pub(crate) async fn submit_gateway_statuses(
        &self,
        timestamp: UnixTimestamp,
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

            // insert ipv4 status
            sqlx::query!(
                r#"
                    INSERT INTO gateway_ipv4_status (gateway_details_id, up, timestamp) VALUES (?, ?, ?);
                "#,
                gateway_id,
                gateway_result.working_ipv4,
                timestamp
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
                timestamp
            )
                .execute(&mut tx)
                .await?;
        }

        // finally commit the transaction
        tx.commit().await
    }

    /// Checks whether there are already any historical uptimes with this particular date.
    pub(crate) async fn check_for_historical_uptime_existence(
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
    pub(crate) async fn insert_mixnode_historical_uptime(
        &self,
        node_id: i64,
        date: &str,
        ipv4_uptime: u8,
        ipv6_uptime: u8,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO mixnode_historical_uptime(mixnode_details_id, date, ipv4_uptime, ipv6_uptime) VALUES (?, ?, ?, ?)",
            node_id,
                date,
                ipv4_uptime,
                ipv6_uptime,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }

    /// Creates new entry for gateway historical uptime
    pub(crate) async fn insert_gateway_historical_uptime(
        &self,
        node_id: i64,
        date: &str,
        ipv4_uptime: u8,
        ipv6_uptime: u8,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO gateway_historical_uptime(gateway_details_id, date, ipv4_uptime, ipv6_uptime) VALUES (?, ?, ?, ?)",
            node_id,
                date,
                ipv4_uptime,
                ipv6_uptime,
            ).execute(&self.connection_pool).await?;
        Ok(())
    }

    /// Creates a database entry for a finished network monitor test run.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: unix timestamp at which the monitor test run has occurred
    pub(crate) async fn insert_monitor_run(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("INSERT INTO monitor_run(timestamp) VALUES (?)", timestamp)
            .execute(&self.connection_pool)
            .await?;
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
        since: UnixTimestamp,
        until: UnixTimestamp,
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

    /// Removes all ipv4 statuses for all mixnodes that are older than the
    /// provided timestamp. This method is indirectly called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_mixnode_ipv4_statuses(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM mixnode_ipv4_status WHERE timestamp < ?",
            timestamp
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Removes all ipv6 statuses for all mixnodes that are older than the
    /// provided timestamp. This method is indirectly called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_mixnode_ipv6_statuses(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM mixnode_ipv6_status WHERE timestamp < ?",
            timestamp
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Removes all ipv4 statuses for all gateways that are older than the
    /// provided timestamp. This method is indirectly called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_gateway_ipv4_statuses(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM gateway_ipv4_status WHERE timestamp < ?",
            timestamp
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Removes all ipv6 statuses for all gateways that are older than the
    /// provided timestamp. This method is indirectly called at every reward cycle.
    ///
    /// # Arguments
    ///
    /// * `until`: timestamp specifying the purge cutoff.
    pub(crate) async fn purge_old_gateway_ipv6_statuses(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM gateway_ipv6_status WHERE timestamp < ?",
            timestamp
        )
        .execute(&self.connection_pool)
        .await?;
        Ok(())
    }

    /// Returns public key, owner and id of all mixnodes that have had any ipv4 statuses submitted
    /// since the provided timestamp.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: indicates the lower bound timestamp for deciding whether given mixnode is active
    pub(crate) async fn get_all_active_mixnodes(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<ActiveNode>, sqlx::Error> {
        // find mixnode details of all nodes that have had at least 1 ipv4 status since the provided
        // timestamp
        // TODO: I dont know if theres a potential issue of if we have a lot of inactive nodes that
        // haven't mixed in ages, they might increase the query times?
        sqlx::query_as!(
            ActiveNode,
            r#"
                SELECT DISTINCT identity, owner, id
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
        .await
    }

    /// Returns public key, owner and id of all gateways that have had any ipv4 statuses submitted
    /// since the provided timestamp.
    ///
    /// # Arguments
    ///
    /// * `timestamp`: indicates the lower bound timestamp for deciding whether given gateway is active
    pub(crate) async fn get_all_active_gateways(
        &self,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<ActiveNode>, sqlx::Error> {
        sqlx::query_as!(
            ActiveNode,
            r#"
                SELECT DISTINCT identity, owner, id
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
        .await
    }

    /// Inserts new rewarding report into the database. Returns id of the newly created entry.
    ///
    /// # Arguments
    ///
    /// * `report`: report to insert into the database
    pub(crate) async fn insert_rewarding_report(
        &self,
        report: RewardingReport,
    ) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!(
            r#"
                INSERT INTO rewarding_report
                (timestamp, eligible_mixnodes, eligible_gateways, possibly_unrewarded_mixnodes, possibly_unrewarded_gateways)
                VALUES (?, ?, ?, ?, ?);
            "#,
            report.timestamp,
            report.eligible_mixnodes,
            report.eligible_gateways,
            report.possibly_unrewarded_mixnodes,
            report.possibly_unrewarded_gateways,
        )
            .execute(&self.connection_pool)
            .await?;

        Ok(res.last_insert_rowid())
    }

    /// Tries to obtain the most recent rewarding report currently stored.
    ///
    /// Returns None if no report exists.
    pub(crate) async fn get_most_recent_rewarding_report(
        &self,
    ) -> Result<Option<RewardingReport>, sqlx::Error> {
        sqlx::query!(
            r#"
                SELECT timestamp, eligible_mixnodes, eligible_gateways, possibly_unrewarded_mixnodes, possibly_unrewarded_gateways
                FROM rewarding_report
                ORDER BY timestamp DESC
                LIMIT 1
            "#,
        )
            .fetch_optional(&self.connection_pool)
            .await.map(|optional_row| optional_row.map(|row| RewardingReport {
            timestamp: row.timestamp,
            eligible_mixnodes: row.eligible_mixnodes,
            eligible_gateways: row.eligible_gateways,
            possibly_unrewarded_mixnodes: row.possibly_unrewarded_mixnodes,
            possibly_unrewarded_gateways: row.possibly_unrewarded_gateways,
        }))
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
    ) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!(
            r#"
                INSERT INTO failed_mixnode_reward_chunk (error_message, reward_summary_id) VALUES (?, ?)
            "#,
            failed_chunk.error_message,
            failed_chunk.report_id,
        ).execute(&self.connection_pool).await?;

        Ok(res.last_insert_rowid())
    }

    /// Inserts new failed gateway reward chunk information into the database.
    /// Returns id of the newly created entry.
    ///
    /// # Arguments
    ///
    /// * `failed_chunk`: chunk information to insert.
    pub(crate) async fn insert_failed_gateway_reward_chunk(
        &self,
        failed_chunk: FailedGatewayRewardChunk,
    ) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!(
            r#"
                INSERT INTO failed_gateway_reward_chunk (error_message, reward_summary_id) VALUES (?, ?)
            "#,
            failed_chunk.error_message,
            failed_chunk.report_id,
        ).execute(&self.connection_pool).await?;

        Ok(res.last_insert_rowid())
    }

    /// Inserts information into the database about a mixnode that might have been unfairly unrewarded this epoch.
    ///
    /// # Arguments
    ///
    /// * `mixnode`: mixnode information to insert.
    pub(crate) async fn insert_possibly_unrewarded_mixnode(
        &self,
        mixnode: PossiblyUnrewardedMixnode,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO possibly_unrewarded_mixnode (identity, uptime, failed_mixnode_reward_chunk_id) VALUES (?, ?, ?)
            "#,
            mixnode.identity,
            mixnode.uptime,
            mixnode.chunk_id
        ).execute(&self.connection_pool).await?;
        Ok(())
    }

    /// Inserts information into the database about a gateway that might have been unfairly unrewarded this epoch.
    ///
    /// # Arguments
    ///
    /// * `gateway`: mixnode information to insert.
    pub(crate) async fn insert_possibly_unrewarded_gateway(
        &self,
        gateway: PossiblyUnrewardedGateway,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
                INSERT INTO possibly_unrewarded_gateway (identity, uptime, failed_gateway_reward_chunk_id) VALUES (?, ?, ?)
            "#,
            gateway.identity,
            gateway.uptime,
            gateway.chunk_id
        ).execute(&self.connection_pool).await?;
        Ok(())
    }

    pub(crate) async fn get_all_active_mixnodes_statuses(
        &self,
        since: UnixTimestamp,
    ) -> Result<Vec<ActiveNodeDayStatuses>, sqlx::Error> {
        let active_nodes = self.get_all_active_mixnodes(since).await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for active_node in active_nodes.into_iter() {
            let ipv4_statuses = self
                .get_mixnode_ipv4_statuses_since_by_id(active_node.id, since)
                .await?;
            let ipv6_statuses = self
                .get_mixnode_ipv6_statuses_since_by_id(active_node.id, since)
                .await?;

            let statuses = ActiveNodeDayStatuses {
                identity: active_node.identity,
                owner: active_node.owner,
                ipv4_statuses,
                ipv6_statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }

    pub(crate) async fn get_all_active_gateways_statuses(
        &self,
        since: UnixTimestamp,
    ) -> Result<Vec<ActiveNodeDayStatuses>, sqlx::Error> {
        let active_nodes = self.get_all_active_gateways(since).await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for active_node in active_nodes.into_iter() {
            let ipv4_statuses = self
                .get_gateway_ipv4_statuses_since_by_id(active_node.id, since)
                .await?;
            let ipv6_statuses = self
                .get_gateway_ipv6_statuses_since_by_id(active_node.id, since)
                .await?;

            let statuses = ActiveNodeDayStatuses {
                identity: active_node.identity,
                owner: active_node.owner,
                ipv4_statuses,
                ipv6_statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }
}
