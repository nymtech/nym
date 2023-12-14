// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::rewarder::epoch::Epoch;
use sqlx::types::time::OffsetDateTime;
use sqlx::{Executor, Sqlite};
use tracing::{instrument, trace};

#[derive(Clone)]
pub(crate) struct StorageManager {
    pub(crate) connection_pool: sqlx::SqlitePool,
}

impl StorageManager {
    pub(crate) async fn insert_rewarding_epoch(
        &self,
        epoch: Epoch,
        rewarding_budget: String,
        rewarding_tx: Option<String>,
        rewarding_error: Option<String>,
    ) -> Result<(), sqlx::Error> {
        todo!()
    }

    pub(crate) async fn insert_rewarding_epoch_block_signing(
        &self,
        epoch: i64,
        total_voting_power_at_epoch_start: i64,
        num_blocks: i64,
        budget: String,
    ) -> Result<(), sqlx::Error> {
        todo!()
    }

    pub(crate) async fn insert_rewarding_epoch_block_signing_reward(
        &self,
    ) -> Result<(), sqlx::Error> {
        todo!()
    }

    pub(crate) async fn insert_rewarding_epoch_credential_issuance(
        &self,
    ) -> Result<(), sqlx::Error> {
        todo!()
    }

    pub(crate) async fn insert_rewarding_epoch_credential_issuance_reward(
        &self,
    ) -> Result<(), sqlx::Error> {
        todo!()
    }
}
