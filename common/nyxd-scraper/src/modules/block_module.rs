// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::FullBlockInformation;
use crate::error::ScraperError;
use crate::storage::StorageTransaction;
use async_trait::async_trait;

#[async_trait]
pub trait BlockModule {
    async fn handle_block(
        &mut self,
        block: &FullBlockInformation,
        storage_tx: &mut StorageTransaction,
    ) -> Result<(), ScraperError>;
}
