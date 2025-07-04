// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::PostgresScraperError;
use crate::storage::manager::{
    insert_block, insert_message, insert_precommit, insert_transaction, insert_validator,
};
use async_trait::async_trait;
use nyxd_scraper_shared::helpers::{
    validator_consensus_address, validator_info, validator_pubkey_to_bech32,
};
use nyxd_scraper_shared::storage::validators::Response;
use nyxd_scraper_shared::storage::{
    validators, Block, Commit, CommitSig, NyxdScraperStorageError, NyxdScraperTransaction,
};
use nyxd_scraper_shared::ParsedTransactionResponse;
use sqlx::{Postgres, Transaction};
use std::ops::{Deref, DerefMut};
use tracing::{debug, trace, warn};

pub struct PostgresStorageTransaction(pub(crate) Transaction<'static, Postgres>);

impl Deref for PostgresStorageTransaction {
    type Target = Transaction<'static, Postgres>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PostgresStorageTransaction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PostgresStorageTransaction {
    async fn persist_validators(
        &mut self,
        validators: &validators::Response,
    ) -> Result<(), PostgresScraperError> {
        debug!("persisting {} validators", validators.total);
        for validator in &validators.validators {
            let consensus_address = validator_consensus_address(validator.address)?;
            let consensus_pubkey = validator_pubkey_to_bech32(validator.pub_key)?;

            insert_validator(
                consensus_address.to_string(),
                consensus_pubkey.to_string(),
                self.0.as_mut(),
            )
            .await?;
        }

        Ok(())
    }

    async fn persist_block_data(
        &mut self,
        block: &Block,
        total_gas: i64,
    ) -> Result<(), PostgresScraperError> {
        let proposer_address =
            validator_consensus_address(block.header.proposer_address)?.to_string();

        insert_block(
            block.header.height.into(),
            block.header.hash().to_string(),
            block.data.len() as u32,
            total_gas,
            proposer_address,
            block.header.time.into(),
            self.0.as_mut(),
        )
        .await?;
        Ok(())
    }

    async fn persist_commits(
        &mut self,
        commits: &Commit,
        validators: &validators::Response,
    ) -> Result<(), PostgresScraperError> {
        debug!("persisting up to {} commits", commits.signatures.len());
        let height: i64 = commits.height.into();

        for commit_sig in &commits.signatures {
            let (validator_id, timestamp, signature) = match commit_sig {
                CommitSig::BlockIdFlagAbsent => {
                    trace!("absent signature");
                    continue;
                }
                CommitSig::BlockIdFlagCommit {
                    validator_address,
                    timestamp,
                    signature,
                } => (validator_address, timestamp, signature),
                CommitSig::BlockIdFlagNil {
                    validator_address,
                    timestamp,
                    signature,
                } => (validator_address, timestamp, signature),
            };

            let validator = validator_info(*validator_id, validators)?;
            let validator_address = validator_consensus_address(*validator_id)?;

            if signature.is_none() {
                warn!("empty signature for {validator_address} at height {height}");
                continue;
            }

            insert_precommit(
                validator_address.to_string(),
                height,
                (*timestamp).into(),
                validator.power.into(),
                validator.proposer_priority.value(),
                self.0.as_mut(),
            )
            .await?;
        }

        Ok(())
    }

    async fn persist_txs(
        &mut self,
        txs: &[ParsedTransactionResponse],
    ) -> Result<(), PostgresScraperError> {
        debug!("persisting {} txs", txs.len());

        for chain_tx in txs {
            insert_transaction(
                chain_tx.hash.to_string(),
                chain_tx.height.into(),
                chain_tx.index as i64,
                chain_tx.tx_result.code.is_ok(),
                chain_tx.tx.body.messages.len() as i64,
                chain_tx.tx.body.memo.clone(),
                chain_tx.tx_result.gas_wanted,
                chain_tx.tx_result.gas_used,
                chain_tx.tx_result.log.clone(),
                self.0.as_mut(),
            )
            .await?;
        }

        Ok(())
    }

    async fn persist_messages(
        &mut self,
        txs: &[ParsedTransactionResponse],
    ) -> Result<(), PostgresScraperError> {
        debug!("persisting messages");

        for chain_tx in txs {
            for (index, msg) in chain_tx.tx.body.messages.iter().enumerate() {
                insert_message(
                    chain_tx.hash.to_string(),
                    index as i64,
                    msg.type_url.clone(),
                    chain_tx.height.into(),
                    self.0.as_mut(),
                )
                .await?
            }
        }

        Ok(())
    }
}

#[async_trait]
impl NyxdScraperTransaction for PostgresStorageTransaction {
    async fn commit(self) -> Result<(), NyxdScraperStorageError> {
        self.0
            .commit()
            .await
            .map_err(PostgresScraperError::from)
            .map_err(NyxdScraperStorageError::from)
    }

    async fn persist_validators(
        &mut self,
        validators: &Response,
    ) -> Result<(), NyxdScraperStorageError> {
        self.persist_validators(validators)
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn persist_block_data(
        &mut self,
        block: &Block,
        total_gas: i64,
    ) -> Result<(), NyxdScraperStorageError> {
        self.persist_block_data(block, total_gas)
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn persist_commits(
        &mut self,
        commits: &Commit,
        validators: &Response,
    ) -> Result<(), NyxdScraperStorageError> {
        self.persist_commits(commits, validators)
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn persist_txs(
        &mut self,
        txs: &[ParsedTransactionResponse],
    ) -> Result<(), NyxdScraperStorageError> {
        self.persist_txs(txs)
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn persist_messages(
        &mut self,
        txs: &[ParsedTransactionResponse],
    ) -> Result<(), NyxdScraperStorageError> {
        self.persist_messages(txs)
            .await
            .map_err(NyxdScraperStorageError::from)
    }

    async fn update_last_processed(&mut self, height: i64) -> Result<(), NyxdScraperStorageError> {
        self.update_last_processed(height)
            .await
            .map_err(NyxdScraperStorageError::from)
    }
}
