// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::PostgresScraperError;
use crate::storage::helpers::parse_addresses_from_events;
use crate::storage::manager::{
    insert_block, insert_message, insert_precommit, insert_transaction, insert_validator,
    update_last_processed,
};
use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose;
use cosmrs::proto;
use nyxd_scraper_shared::ParsedTransactionResponse;
use nyxd_scraper_shared::helpers::{
    validator_consensus_address, validator_info, validator_pubkey_to_bech32,
};
use nyxd_scraper_shared::storage::validators::Response;
use nyxd_scraper_shared::storage::{
    Block, Commit, CommitSig, NyxdScraperStorageError, NyxdScraperTransaction, validators,
};
use serde_json::json;
use sqlx::types::time::{OffsetDateTime, PrimitiveDateTime};
use sqlx::{Postgres, Transaction};
use std::ops::{Deref, DerefMut};
use tracing::{debug, error, trace, warn};

pub struct PostgresStorageTransaction {
    pub(super) inner: Transaction<'static, Postgres>,
}

impl Deref for PostgresStorageTransaction {
    type Target = Transaction<'static, Postgres>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PostgresStorageTransaction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
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
                self.inner.as_mut(),
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

        let offset_datetime: OffsetDateTime = block.header.time.into();
        let time = PrimitiveDateTime::new(offset_datetime.date(), offset_datetime.time());

        insert_block(
            block.header.height.into(),
            block.header.hash().to_string(),
            block.data.len() as i32,
            total_gas,
            proposer_address,
            time,
            self.inner.as_mut(),
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

            let offset_datetime: OffsetDateTime = (*timestamp).into();
            let time = PrimitiveDateTime::new(offset_datetime.date(), offset_datetime.time());

            insert_precommit(
                validator_address.to_string(),
                height,
                time,
                validator.power.into(),
                validator.proposer_priority.value(),
                self.inner.as_mut(),
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
            // bdjuno style, base64 encode them
            let signatures = chain_tx
                .tx
                .signatures
                .iter()
                .map(|sig| general_purpose::STANDARD.encode(sig))
                .collect();

            let messages = chain_tx
                .parsed_messages
                .values()
                .cloned()
                .collect::<Vec<_>>();

            let signer_infos = chain_tx
                .tx
                .auth_info
                .signer_infos
                .iter()
                .map(|info| proto::cosmos::tx::v1beta1::SignerInfo::from(info.clone()))
                .collect::<Vec<_>>();

            let hash = chain_tx.hash.to_string();
            let height = chain_tx.height.into();
            let index = chain_tx.index as i32;

            let log = serde_json::to_value(chain_tx.tx_result.log.clone()).map_err(|e| error!(hash, height, index, "Failed to parse logs: {e}")).unwrap_or_default();
            let events = &chain_tx.tx_result.events;

            insert_transaction(
                hash,
                height,
                index,
                chain_tx.tx_result.code.is_ok(),
                serde_json::Value::Array(messages),
                chain_tx.tx.body.memo.clone(),
                signatures,
                serde_json::to_value(signer_infos)?,
                serde_json::to_value(&chain_tx.tx.auth_info.fee)?,
                chain_tx.tx_result.gas_wanted,
                chain_tx.tx_result.gas_used,
                chain_tx.tx_result.log.clone(),
                json!(log),
                json!(events),
                self.inner.as_mut(),
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
            let involved_addresses = parse_addresses_from_events(chain_tx);
            for (index, msg) in chain_tx.tx.body.messages.iter().enumerate() {
                let parsed_message = chain_tx.parsed_messages.get(&index);
                let value = serde_json::to_value(parsed_message)?;

                insert_message(
                    chain_tx.hash.to_string(),
                    index as i64,
                    msg.type_url.clone(),
                    value,
                    involved_addresses.clone(),
                    chain_tx.height.into(),
                    self.inner.as_mut(),
                )
                .await?
            }
        }

        Ok(())
    }

    async fn update_last_processed(&mut self, height: i64) -> Result<(), PostgresScraperError> {
        debug!("update_last_processed");
        update_last_processed(height, self.inner.as_mut()).await?;
        Ok(())
    }
}

#[async_trait]
impl NyxdScraperTransaction for PostgresStorageTransaction {
    async fn commit(self) -> Result<(), NyxdScraperStorageError> {
        self.inner
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
