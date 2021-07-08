// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    GatewayStatusReport, MixnodeStatusReport, NodeStatusApiError,
};
// use diesel::prelude::*;
// use diesel::r2d2;
use futures::TryStreamExt;
use rocket::fairing::{self, AdHoc, Fairing, Info};
use rocket::response::Debug;
use rocket::serde::json::Json;
use rocket::{Build, Data, Orbit, Request, Response, Rocket, State};
// use rocket_sync_db_pools::diesel;
// use rocket_sync_db_pools::rocket::response::status::Created;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono;
use sqlx::ConnectOptions;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use topology::mix::Node;

// #[database("diesel")]
// pub struct DieselDb(diesel::SqliteConnection);
//
// #[derive(Clone)]
// pub struct SqlxDb(pub sqlx::SqlitePool);
//
// #[derive(Debug, Clone, Deserialize, Serialize)]
// #[serde(crate = "rocket::serde")]
// pub struct SqlxPost {
//     #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
//     pub id: Option<i64>,
//     pub title: String,
//     pub text: String,
//     // pub timestamp: i64,
// }
//
// #[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable)]
// #[serde(crate = "rocket::serde")]
// #[table_name = "posts"]
// pub struct DieselPost {
//     #[serde(skip_deserializing)]
//     pub id: Option<i32>,
//     pub title: String,
//     pub text: String,
//     #[serde(skip_deserializing)]
//     pub published: bool,
// }
//
// table! {
//     posts (id) {
//         id -> Nullable<Integer>,
//         title -> Text,
//         text -> Text,
//         published -> Bool,
//     }
// }
//
// type DieselResult<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;
//
// #[get("/<id>")]
// async fn diesel_add(db: DieselDb, id: i32) -> DieselResult<Created<Json<DieselPost>>> {
//     let new_post = DieselPost {
//         id: Some(id),
//         title: "aaaa".to_string(),
//         text: "bbbb".to_string(),
//         published: false,
//     };
//     let new_post2 = new_post.clone();
//     db.run(move |conn| {
//         diesel::insert_into(posts::table)
//             .values(new_post)
//             .execute(conn)
//     })
//     .await
//     .unwrap();
//
//     Ok(Created::new("/").body(Json(new_post2)))
// }
//
// #[get("/")]
// async fn diesel_list(db: DieselDb) -> DieselResult<Json<Vec<Option<i32>>>> {
//     let ids: Vec<Option<i32>> = db
//         .run(move |conn| posts::table.select(posts::id).load(conn))
//         .await?;
//
//     Ok(Json(ids))
// }
//
// async fn run_diesel_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
//     // This macro from `diesel_migrations` defines an `embedded_migrations`
//     // module containing a function named `run`. This allows the example to be
//     // run and tested without any outside setup of the database.
//     embed_migrations!("db/migrations/diesel");
//
//     let conn = DieselDb::get_one(&rocket)
//         .await
//         .expect("database connection");
//     conn.run(|c| embedded_migrations::run(c))
//         .await
//         .expect("can run migrations");
//
//     rocket
// }

//
// type SqlxResult<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;
//
// #[get("/<id>")]
// async fn sqlx_add(db: &State<SqlxDb>, id: i64) -> SqlxResult<Created<Json<SqlxPost>>> {
//     let new_post = SqlxPost {
//         id: Some(id),
//         title: "aaaaaddd".to_string(),
//         text: "bbbbbbbbb".to_string(),
//     };
//
//     let q1 = sqlx::query!(
//         "INSERT INTO posts (id, title, text) VALUES (?, ?, ?)",
//         new_post.id,
//         new_post.title,
//         new_post.text
//     );
//     let foo = Some(id + 1);
//
//     let q2 = sqlx::query!(
//         "INSERT INTO posts (id, title, text) VALUES (?, ?, ?)",
//         foo,
//         new_post.title,
//         new_post.text
//     );
//
//     let mut tx = db.0.begin().await.unwrap();
//     q1.execute(&mut tx).await?;
//     q2.execute(&mut tx).await?;
//     tx.commit().await.unwrap();
//
//     Ok(Created::new("/").body(Json(new_post)))
//
//     // let new_post = DieselPost {
//     //     id: Some(id),
//     //     title: "aaaa".to_string(),
//     //     text: "bbbb".to_string(),
//     //     published: false,
//     // };
//     // let new_post2 = new_post.clone();
//     // db.run(move |conn| {
//     //     diesel::insert_into(posts::table)
//     //         .values(new_post)
//     //         .execute(conn)
//     // })
//     //     .await
//     //     .unwrap();
//     //
//     // Ok(Created::new("/").body(Json(new_post2)))
// }
//
// #[get("/")]
// async fn sqlx_list(db: &State<SqlxDb>) -> SqlxResult<Json<Vec<i64>>> {
//     let ids = sqlx::query!("SELECT id FROM posts")
//         .fetch(&db.0)
//         .map_ok(|record| record.id)
//         .try_collect::<Vec<_>>()
//         .await?;
//
//     Ok(Json(ids))
// }
//
// async fn init_sqlx_db(rocket: Rocket<Build>) -> fairing::Result {
//     use rocket_sync_db_pools::Config;
//
//     let config = match Config::from("sqlx", &rocket) {
//         Ok(config) => config,
//         Err(e) => {
//             error!("Failed to read SQLx config: {}", e);
//             return Err(rocket);
//         }
//     };
//
//     let mut opts = sqlx::sqlite::SqliteConnectOptions::new()
//         .filename(&config.url)
//         .create_if_missing(true);
//
//     opts.disable_statement_logging();
//     let db = match sqlx::SqlitePool::connect_with(opts).await {
//         Ok(db) => db,
//         Err(e) => {
//             error!("Failed to connect to SQLx database: {}", e);
//             return Err(rocket);
//         }
//     };
//
//     if let Err(e) = sqlx::migrate!("db/migrations/sqlx").run(&db).await {
//         error!("Failed to initialize SQLx database: {}", e);
//         return Err(rocket);
//     }
//
//     Ok(rocket.manage(SqlxDb(db)))
// }
//
// pub fn stage() -> AdHoc {
//     todo!()
//     // let foo = NodeStatusStorage::fairing();
//     // println!("{:?}", foo);
//     // AdHoc::on_ignite("Diesel SQLite Stage", |rocket| async {
//     //     rocket
//     //         .attach(NodeStatusStorage::fairing())
//     //         .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
//     //         .mount("/diesel", routes![foo, list])
//     // })
// }
//
// pub fn stage_sqlx() -> AdHoc {
//     AdHoc::on_ignite("SQLx Stage", |rocket| async {
//         rocket
//             .attach(AdHoc::try_on_ignite("SQLx Database", init_sqlx_db))
//             .mount("/sqlx", routes![sqlx_add, sqlx_list])
//     })
// }
//
// pub fn stage_diesel() -> AdHoc {
//     AdHoc::on_ignite("Diesel SQLite Stage", |rocket| async {
//         rocket
//             .attach(DieselDb::fairing())
//             .attach(AdHoc::on_ignite("Diesel Migrations", run_diesel_migrations))
//             .mount("/diesel", routes![diesel_add, diesel_list])
//     })
// }

//
// impl NodeStatusStorage {
//     pub(crate) fn new() -> Self {
//         todo!()
//         // NodeStatusStorage {
//         //     inner: Arc::new(RwLock::new(Inner::new())),
//         // }
//     }
//
// }
//
// struct Inner {
//     //
// }
//
// impl Inner {
//     fn new() -> Self {
//         Inner {}
//     }
// }

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
