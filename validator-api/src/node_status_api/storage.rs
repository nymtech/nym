// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport, MixnodeUptimeHistory,
    NodeStatusApiError, StatusReport, Uptime,
};
use rocket::fairing::{self, AdHoc};
use rocket::{Build, Rocket};
use sqlx::ConnectOptions;
// use std::fmt::{self, Display, Formatter};
use crate::network_monitor::monitor::summary_producer::NodeResult;
use crate::node_status_api::ONE_DAY;
use sqlx::types::time;
use sqlx::types::time::OffsetDateTime;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub(crate) struct NodeStatusStorage {
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

        let storage = NodeStatusStorage { connection_pool };

        Ok(rocket.manage(storage))
    }

    pub(crate) fn stage() -> AdHoc {
        AdHoc::try_on_ignite("SQLx Database", NodeStatusStorage::init)
    }

    pub(crate) async fn get_mixnode_ipv4_statuses_since(
        &self,
        identity: &str,
        timestamp: i64,
    ) -> Result<Vec<StatusReport>, NodeStatusApiError> {
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
                return Err(NodeStatusApiError::InternalDatabaseError);
            }
        }.into_iter().map(|row| StatusReport { timestamp: row.timestamp, up: row.up }).collect();

        Ok(reports)
    }

    pub(crate) async fn get_mixnode_ipv6_statuses_since(
        &self,
        identity: &str,
        timestamp: i64,
    ) -> Result<Vec<StatusReport>, NodeStatusApiError> {
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
                return Err(NodeStatusApiError::InternalDatabaseError);
            }
        }.into_iter().map(|row| StatusReport { timestamp: row.timestamp, up: row.up }).collect();

        Ok(reports)
    }

    pub(crate) async fn get_gateway_ipv4_statuses_since(
        &self,
        identity: &str,
        timestamp: i64,
    ) -> Result<Vec<StatusReport>, NodeStatusApiError> {
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
                return Err(NodeStatusApiError::InternalDatabaseError);
            }
        }.into_iter().map(|row| StatusReport { timestamp: row.timestamp, up: row.up }).collect();

        Ok(reports)
    }

    pub(crate) async fn get_gateway_ipv6_statuses_since(
        &self,
        identity: &str,
        timestamp: i64,
    ) -> Result<Vec<StatusReport>, NodeStatusApiError> {
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
                return Err(NodeStatusApiError::InternalDatabaseError);
            }
        }.into_iter().map(|row| StatusReport { timestamp: row.timestamp, up: row.up }).collect();

        Ok(reports)
    }

    pub(crate) async fn get_mixnode_statuses(&self, identity: &str) {
        // WHERE mixnode_details.pub_key="node1" AND mixnode_ipv4_status.timestamp > 1625742013602;

        // for time being just get them all
        sqlx::query!(
            r#"
            SELECT up
                FROM mixnode_ipv4_status
                JOIN mixnode_details
                ON mixnode_ipv4_status.mixnode_details_id = mixnode_details.id
                WHERE mixnode_details.pub_key=?;
            "#,
            identity
        )
        .fetch_all(&self.connection_pool)
        .await
        .unwrap()
        .into_iter()
        .for_each(|r| println!("{}", r.up));

        // println!("all ups: {:?}", ups);
    }

    pub(crate) async fn get_mixnode_report(
        &self,
        identity: &str,
    ) -> Result<MixnodeStatusReport, NodeStatusApiError> {
        let now = OffsetDateTime::now_utc();
        let day_ago = now - ONE_DAY;

        let ipv4_statuses = self
            .get_mixnode_ipv4_statuses_since(identity, day_ago.unix_timestamp())
            .await?;

        // if we have no statuses, the node doesn't exist (or monitor is down), but either way, we can't make a report
        if ipv4_statuses.is_empty() {
            return Err(NodeStatusApiError::MixnodeReportNotFound(
                identity.to_owned(),
            ));
        }

        let ipv6_statuses = self
            .get_mixnode_ipv6_statuses_since(identity, day_ago.unix_timestamp())
            .await?;

        // now, technically this is not a critical error, but this should have NEVER happened in the first place
        // so something super weird is going on
        if ipv4_statuses.len() != ipv6_statuses.len() {
            error!("Somehow we have different number of ipv4 and ipv6 statuses for mixnode {} in range {} - {}! (ipv4: {}, ipv6: {})",
            identity,
                day_ago.unix_timestamp(),
                now.unix_timestamp(),
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

    pub(crate) async fn get_gateway_report(
        &self,
        identity: &str,
    ) -> Result<GatewayStatusReport, NodeStatusApiError> {
        Err(NodeStatusApiError::GatewayReportNotFound(
            identity.to_string(),
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

    // provisional API for network monitor
    pub(crate) async fn submit_new_statuses(
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

            // TODO: the potential "problem" (if you can call it that way) is that if entry DID exist
            // then the id field will be incremented for the next node we create thus we will
            // have gaps in our ids. ask @DH if that's fine (I don't see why not because nodes
            // are still correctly ordered and you can get their total number with a simple query
            // and we'd have to run the system until the heat death of the universe to run out of id numbers)
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

    pub(crate) async fn make_up_mixnode(&self, identity: &str) {
        let owner = format!("foomper-{}", identity);
        sqlx::query!(
            "INSERT INTO mixnode_details (owner, pub_key) VALUES (?, ?)",
            owner,
            identity
        )
        .execute(&self.connection_pool)
        .await
        .unwrap();
    }

    pub(crate) async fn add_up_status(&self, identity: &str) {
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();

        sqlx::query!(
            r#"
            INSERT INTO mixnode_ipv4_status (mixnode_details_id, up, timestamp)
                SELECT mixnode_details.id, ?, ?
                FROM mixnode_details
                WHERE mixnode_details.pub_key=?
            "#,
            true,
            timestamp,
            identity
        )
        .execute(&self.connection_pool)
        .await
        .unwrap();
    }

    pub(crate) async fn add_down_status(&self, identity: &str) {
        let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();

        sqlx::query!(
            r#"
            INSERT INTO mixnode_ipv4_status (mixnode_details_id, up, timestamp)
                SELECT mixnode_details.id, ?, ?
                FROM mixnode_details
                WHERE mixnode_details.pub_key=?
            "#,
            false,
            timestamp,
            identity
        )
        .execute(&self.connection_pool)
        .await
        .unwrap();
    }

    /*
        INSERT INTO orders ( userid, timestamp)
    SELECT o.userid , o.timestamp FROM users u INNER JOIN orders o ON  o.userid = u.id
         */

    // pub(crate) async fn make_up_status(&self, identity: &str) {
    //     let now = SystemTime::now()
    //         .duration_since(UNIX_EPOCH)
    //         .expect("The time went backwards - congratulation on creating the time machine!");
    //     let now_nanos = now.as_nanos();
    //
    //     sqlx::query!("INSERT INTO aa (idVALUES (")
    // }
}
