// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport, MixnodeUptimeHistory,
    NodeStatusApiError,
};
use rocket::fairing::{self, AdHoc};
use rocket::{Build, Rocket};
use sqlx::ConnectOptions;
// use std::fmt::{self, Display, Formatter};
use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::node_status_api::utils::NodeStatus;
use crate::node_status_api::ONE_DAY;
use sqlx::types::time::OffsetDateTime;
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
            identity,
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
            identity,
            ipv4_statuses,
            ipv6_statuses,
        ))
    }

    pub(crate) async fn get_mixnode_uptime_history(
        &self,
        identity: &str,
    ) -> Result<MixnodeUptimeHistory, NodeStatusApiError> {
        todo!()
    }

    pub(crate) async fn get_gateway_uptime_history(
        &self,
        identity: &str,
    ) -> Result<GatewayUptimeHistory, NodeStatusApiError> {
        todo!()
    }

    pub(crate) async fn get_all_mixnode_reports(
        &self,
    ) -> Result<Vec<MixnodeStatusReport>, NodeStatusApiError> {
        todo!()
    }

    pub(crate) async fn get_all_gateway_reports(
        &self,
    ) -> Result<Vec<GatewayStatusReport>, NodeStatusApiError> {
        todo!()
    }

    // NETWORK MONITOR API

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

    pub(crate) async fn update_historical_uptimes(&self) -> Result<(), sqlx::Error> {
        let today_iso_8601 = OffsetDateTime::now_utc().date().to_string();

        // let mut tx = self.connection_pool.begin().await?;

        Ok(())
    }

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
        let reports = match sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv4_status
                    JOIN mixnode_details
                    ON mixnode_ipv4_status.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.pub_key=?;
                "#,
            identity
        )
            .fetch_all(&self.connection_pool)
            .await
        {
            Ok(records) => records,
            Err(err) => {
                error!("The database run into problems while trying to get all ipv4 uptimes for mixnode {} since {}. Error: {}", identity, timestamp, err);
                return Err(err);
            }
        }.into_iter().map(|row| NodeStatus { timestamp: row.timestamp, up: row.up }).collect();

        Ok(reports)
    }

    /// Gets all ipv6 statuses for mixnode with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_mixnode_ipv6_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = match sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM mixnode_ipv6_status
                    JOIN mixnode_details
                    ON mixnode_ipv6_status.mixnode_details_id = mixnode_details.id
                    WHERE mixnode_details.pub_key=?;
                "#,
            identity
        )
            .fetch_all(&self.connection_pool)
            .await
        {
            Ok(records) => records,
            Err(err) => {
                error!("The database run into problems while trying to get all ipv6 uptimes for mixnode {} since {}. Error: {}", identity, timestamp, err);
                return Err(err);
            }
        }.into_iter().map(|row| NodeStatus { timestamp: row.timestamp, up: row.up }).collect();

        Ok(reports)
    }

    /// Gets all ipv4 statuses for gateway with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_gateway_ipv4_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = match sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv4_status
                    JOIN gateway_details
                    ON gateway_ipv4_status.gateway_details_id = gateway_details.id
                    WHERE gateway_details.pub_key=?;
                "#,
            identity
        )
            .fetch_all(&self.connection_pool)
            .await
        {
            Ok(records) => records,
            Err(err) => {
                error!("The database run into problems while trying to get all ipv4 uptimes for gateway {} since {}. Error: {}", identity, timestamp, err);
                return Err(err);
            }
        }.into_iter().map(|row| NodeStatus { timestamp: row.timestamp, up: row.up }).collect();

        Ok(reports)
    }

    /// Gets all ipv6 statuses for gateway with particular identity that were inserted
    /// into the database after the specified unix timestamp.
    async fn get_gateway_ipv6_statuses_since(
        &self,
        identity: &str,
        timestamp: UnixTimestamp,
    ) -> Result<Vec<NodeStatus>, sqlx::Error> {
        let reports = match sqlx::query!(
            r#"
                SELECT timestamp, up
                    FROM gateway_ipv6_status
                    JOIN gateway_details
                    ON gateway_ipv6_status.gateway_details_id = gateway_details.id
                    WHERE gateway_details.pub_key=?;
                "#,
            identity
        )
            .fetch_all(&self.connection_pool)
            .await
        {
            Ok(records) => records,
            Err(err) => {
                error!("The database run into problems while trying to get all ipv6 uptimes for gateway {} since {}. Error: {}", identity, timestamp, err);
                return Err(err);
            }
        }.into_iter().map(|row| NodeStatus { timestamp: row.timestamp, up: row.up }).collect();

        Ok(reports)
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

    /// Removes all statuses from the databaase that are older than 48h.
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
