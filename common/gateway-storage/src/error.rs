// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayStorageError {
    #[error("Database experienced an internal error: {0}")]
    InternalDatabaseError(#[from] sqlx::Error),

    #[error("Failed to perform database migration: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),

    #[error("could not find any valid shared keys for under id {id}")]
    MissingSharedKey { id: i64 },

    #[error("Somehow stored data is incorrect: {0}")]
    DataCorruption(String),

    #[error("the stored data associated with ticket {ticket_id} is malformed!")]
    MalformedStoredTicketData { ticket_id: i64 },

    #[error("Failed to convert from type of database: {0}")]
    TypeConversion(String),
}
