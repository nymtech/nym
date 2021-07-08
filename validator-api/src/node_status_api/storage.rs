// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    GatewayStatusReport, MixnodeStatusReport, NodeStatusApiError,
};
use rocket::fairing::{self, AdHoc};
use rocket::{Build, Rocket};
use sqlx::ConnectOptions;
// use std::fmt::{self, Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

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
        AdHoc::on_ignite("SQLx Stage", |rocket| async {
            rocket
                .attach(AdHoc::try_on_ignite(
                    "SQLx Database",
                    NodeStatusStorage::init,
                ))
                .mount("/v1/status", routes![])
        })
    }

    pub(crate) async fn get_mixnode_report(
        &self,
        identity: &str,
    ) -> Result<MixnodeStatusReport, NodeStatusApiError> {
        Ok(MixnodeStatusReport::example())
    }

    pub(crate) async fn get_gateway_report(
        &self,
        identity: &str,
    ) -> Result<GatewayStatusReport, NodeStatusApiError> {
        Err(NodeStatusApiError::GatewayReportNotFound(
            identity.to_string(),
        ))
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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("The time went backwards - congratulation on creating a time machine!");

        // since we're working with "normal" timestamps here and not dates arbitrarily far in the future
        // cast to i64 is fine here
        let now_nanos = now.as_millis() as i64;

        sqlx::query!(
            r#"
            INSERT INTO mixnode_ipv4_status (mixnode_details_id, up, timestamp)
                SELECT mixnode_details.id, ?, ?
                FROM mixnode_details
                WHERE mixnode_details.pub_key=?
            "#,
            true,
            now_nanos,
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
