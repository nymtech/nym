// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::str::FromStr;

use crate::models::Client;

#[derive(Debug, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT")] // SQLite TEXT type
pub enum ClientType {
    EntryMixnet,
    EntryWireguard,
    ExitWireguard,
}

impl FromStr for ClientType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entry mixnet" => Ok(ClientType::EntryMixnet),
            "entry wireguard" => Ok(ClientType::EntryWireguard),
            "exit wireguard" => Ok(ClientType::ExitWireguard),
            _ => Err("Invalid client type"),
        }
    }
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ClientType::EntryMixnet => "entry mixnet",
            ClientType::EntryWireguard => "entry wireguard",
            ClientType::ExitWireguard => "exit wireguard",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone)]
pub(crate) struct ClientManager {
    connection_pool: sqlx::SqlitePool,
}

impl ClientManager {
    /// Creates new instance of the `ClientManager` with the provided sqlite connection pool.
    ///
    /// # Arguments
    ///
    /// * `connection_pool`: database connection pool to use.
    pub(crate) fn new(connection_pool: sqlx::SqlitePool) -> Self {
        ClientManager { connection_pool }
    }

    /// Inserts new client to the storage, specifying its type.
    ///
    /// # Arguments
    ///
    /// * `client_type`: Type of the client that gets inserted
    pub(crate) async fn insert_client(&self, client_type: ClientType) -> Result<i64, sqlx::Error> {
        let client_id = sqlx::query!("INSERT INTO clients(client_type) VALUES (?)", client_type)
            .execute(&self.connection_pool)
            .await?
            .last_insert_rowid();
        Ok(client_id)
    }

    /// Tries to retrieve a particular client.
    ///
    /// # Arguments
    ///
    /// * `id`: The client id
    pub(crate) async fn get_client(&self, id: i64) -> Result<Option<Client>, sqlx::Error> {
        sqlx::query_as!(
            Client,
            r#"
            SELECT id, client_type as "client_type: ClientType"
            FROM clients
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.connection_pool)
        .await
    }
}
