// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::node_status_api::models::{HistoricalUptime, Uptime};
use crate::node_status_api::utils::ActiveNodeDayStatuses;
use crate::node_status_api::ONE_DAY;
use crate::storage::models::{ActiveNode, NodeStatus};
use crate::storage::UnixTimestamp;
use sqlx::types::time::OffsetDateTime;
use std::convert::TryFrom;

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(super) connection_pool: sqlx::SqlitePool,
}

// all SQL goes here
impl StorageManager {
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

    /// Creates new entry for gatewy historical uptime
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

    // ####################################################################################################
    // ALL THE METHODS BELOW ARE TEMPORARY AND WILL BE REMOVED ONCE PAYMENTS ARE DONE INSIDE VALIDATOR API
    // ####################################################################################################

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    /// Returns public key, owner and id of all mixnodes that have had any ipv4 statuses submitted
    /// since provided timestamp.
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

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    /// Returns public key, owner and id of all gateways that have had any ipv4 statuses submitted
    /// since provided timestamp.
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

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    // TODO: should that live on the 'Inner' struct or should it rather exist on the actual storage struct
    // since technically it doesn't touch any SQL directly
    pub(crate) async fn get_all_active_mixnodes_statuses(
        &self,
    ) -> Result<Vec<ActiveNodeDayStatuses>, sqlx::Error> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();

        let active_nodes = self.get_all_active_mixnodes(day_ago).await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for active_node in active_nodes.into_iter() {
            let ipv4_statuses = self
                .get_mixnode_ipv4_statuses_since_by_id(active_node.id, day_ago)
                .await?;
            let ipv6_statuses = self
                .get_mixnode_ipv6_statuses_since_by_id(active_node.id, day_ago)
                .await?;

            let statuses = ActiveNodeDayStatuses {
                identity: active_node.identity,
                owner: active_node.owner,
                node_id: active_node.id,
                ipv4_statuses,
                ipv6_statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }

    // NOTE: this method will go away once we move payments into the validator-api
    // it just helps us to get rid of having to query for reports of each node individually
    // TODO: should that live on the 'Inner' struct or should it rather exist on the actual storage struct
    // since technically it doesn't touch any SQL directly
    pub(crate) async fn get_all_active_gateways_statuses(
        &self,
    ) -> Result<Vec<ActiveNodeDayStatuses>, sqlx::Error> {
        let now = OffsetDateTime::now_utc();
        let day_ago = (now - ONE_DAY).unix_timestamp();

        let active_nodes = self.get_all_active_gateways(day_ago).await?;

        let mut active_day_statuses = Vec::with_capacity(active_nodes.len());
        for active_node in active_nodes.into_iter() {
            let ipv4_statuses = self
                .get_gateway_ipv4_statuses_since_by_id(active_node.id, day_ago)
                .await?;
            let ipv6_statuses = self
                .get_gateway_ipv6_statuses_since_by_id(active_node.id, day_ago)
                .await?;

            let statuses = ActiveNodeDayStatuses {
                identity: active_node.identity,
                owner: active_node.owner,
                node_id: active_node.id,
                ipv4_statuses,
                ipv6_statuses,
            };

            active_day_statuses.push(statuses);
        }

        Ok(active_day_statuses)
    }
}
