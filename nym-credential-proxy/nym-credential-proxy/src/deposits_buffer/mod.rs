// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::deposits_buffer::helpers::{request_sizes, BufferedDeposit, PerformedDeposits};
use crate::deposits_buffer::refill_task::RefillTask;
use crate::error::CredentialProxyError;
use crate::http::state::required_deposit_cache::RequiredDepositCache;
use crate::http::state::ChainClient;
use crate::storage::CredentialProxyStorage;
use nym_compact_ecash::PublicKeyUser;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_contract_common::deposit::DepositId;
use nym_validator_client::nyxd::cosmwasm_client::ContractResponseData;
use nym_validator_client::nyxd::Coin;
use rand::rngs::OsRng;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

pub(crate) mod helpers;
mod refill_task;

// TODO: I guess make it configurable
const DEPOSITS_THRESHOLD_P: f32 = 0.1;

struct DepositsBufferInner {
    client: ChainClient,

    required_deposit_cache: RequiredDepositCache,

    storage: CredentialProxyStorage,
    target_amount: usize,
    max_concurrent_deposits: usize,
    unused_deposits: AsyncMutex<Vec<BufferedDeposit>>,

    deposits_refill_task: RefillTask,
    short_sha: &'static str,
    cancellation_token: CancellationToken,
}

#[derive(Clone)]
pub(crate) struct DepositsBuffer {
    inner: Arc<DepositsBufferInner>,
}

impl DepositsBuffer {
    pub(crate) async fn new(
        storage: CredentialProxyStorage,
        client: ChainClient,
        required_deposit_cache: RequiredDepositCache,
        short_sha: &'static str,
        target_amount: usize,
        max_concurrent_deposits: usize,
        cancellation_token: CancellationToken,
    ) -> Result<Self, CredentialProxyError> {
        let unused_deposits = storage.load_unused_deposits().await?;
        info!("managed to load {} deposits", unused_deposits.len());

        Ok(DepositsBuffer {
            inner: Arc::new(DepositsBufferInner {
                client,
                required_deposit_cache,
                storage,
                target_amount,
                max_concurrent_deposits,
                unused_deposits: AsyncMutex::new(unused_deposits),
                deposits_refill_task: RefillTask::default(),
                short_sha,
                cancellation_token,
            }),
        })
    }

    async fn deposit_amount(&self) -> Result<Coin, CredentialProxyError> {
        self.inner
            .required_deposit_cache
            .get_or_update(&self.inner.client)
            .await
    }

    #[instrument(skip(self), err(Display))]
    async fn make_deposits_request(
        &self,
        amount: usize,
    ) -> Result<PerformedDeposits, CredentialProxyError> {
        let requested_on = OffsetDateTime::now_utc();
        let chain_write_permit = self.inner.client.start_chain_tx().await;
        let mut rng = OsRng;

        let deposit_amount = self.deposit_amount().await?;
        let keys = (0..amount)
            .map(|_| ed25519::PrivateKey::new(&mut rng))
            .collect::<Vec<_>>();

        info!("starting {amount} deposits");
        let mut contents = Vec::new();
        for key in &keys {
            let public_key: ed25519::PublicKey = key.into();
            contents.push((public_key.to_base58_string(), deposit_amount.clone()));
        }

        let execute_res = chain_write_permit
            .make_deposits(self.inner.short_sha, contents)
            .await?;

        let tx_hash = execute_res.transaction_hash;
        info!("{amount} deposits made in transaction: {tx_hash}");

        let contract_data = match execute_res.to_contract_data() {
            Ok(contract_data) => contract_data,
            Err(err) => {
                // that one is tricky. deposits technically got made, but we somehow failed to parse response,
                // in this case terminate the proxy with 0 exit code so it wouldn't get automatically restarted
                // because it requires some serious MANUAL intervention
                error!("CRITICAL FAILURE: failed to parse out deposit information from the contract transaction. either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually. error was: {err}");
                self.inner.cancellation_token.cancel();
                return Err(CredentialProxyError::DepositFailure);
            }
        };

        if contract_data.len() != amount {
            // another critical failure, that one should be quite impossible and thus has to be manually inspected
            error!("CRITICAL FAILURE: failed to parse out all deposit information from the contract transaction. got {} responses while we sent {amount} deposits! either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually", contract_data.len());
            self.inner.cancellation_token.cancel();
            return Err(CredentialProxyError::DepositFailure);
        }

        let mut deposits_data = Vec::new();
        for (key, response) in keys.into_iter().zip(contract_data) {
            let response_index = response.message_index;
            let deposit_id = match response.parse_singleton_u32_contract_data() {
                Ok(deposit_id) => deposit_id,
                Err(err) => {
                    // another impossibility
                    error!("CRITICAL FAILURE: failed to parse out deposit id out of the response at index {response_index}: {err}. either the chain got upgraded and the schema changed or the ecash contract got changed! terminating the process. it has to be inspected manually");
                    self.inner.cancellation_token.cancel();
                    return Err(CredentialProxyError::DepositFailure);
                }
            };

            deposits_data.push(BufferedDeposit::new(deposit_id, key));
        }

        Ok(PerformedDeposits {
            deposits_data,
            tx_hash,
            requested_on,
            deposit_amount,
        })
    }

    async fn insert_new_deposits(
        &self,
        mut deposits: PerformedDeposits,
    ) -> Result<(), CredentialProxyError> {
        // 1. insert into the db
        self.inner.storage.insert_new_deposits(&deposits).await?;

        // 2. update the buffer
        self.inner
            .unused_deposits
            .lock()
            .await
            .append(&mut deposits.deposits_data);
        Ok(())
    }

    /// Start refilling our deposit buffer.
    /// It chunks the amount required based on the configured maximum request size
    /// and updates global state after each successful transaction.
    async fn refill_deposits(&self) -> Result<(), CredentialProxyError> {
        let available = self.inner.unused_deposits.lock().await.len();

        let target = self.deposits_upper_threshold();
        let to_request = target - available;

        for request_chunk in request_sizes(to_request, self.inner.max_concurrent_deposits) {
            // note: we check for cancellation between individual requests
            // as opposed to wrapping that in tokio::select! so that we would never abandon chain operations
            // as we wouldn't want to lose funds
            if self.inner.cancellation_token.is_cancelled() {
                info!("received cancellation during deposits refilling");
                return Ok(());
            }

            // make sure to insert deposits into db/vec as we get them so on initial run,
            // we'd start trickling down data as soon as possible
            let deposits = self.make_deposits_request(request_chunk).await?;
            self.insert_new_deposits(deposits).await?;
        }

        Ok(())
    }

    // if we're here, we know we're below the threshold
    fn maybe_refill_deposits(&self) {
        if let Some(mut guard) = self.inner.deposits_refill_task.try_get_new_task_guard() {
            let this = self.clone();
            *guard = Some(tokio::spawn(async move { this.refill_deposits().await }));
        }
    }

    fn deposits_lower_threshold(&self) -> usize {
        self.inner.target_amount - (self.inner.target_amount as f32 * DEPOSITS_THRESHOLD_P) as usize
    }

    fn deposits_upper_threshold(&self) -> usize {
        self.inner.target_amount + (self.inner.target_amount as f32 * DEPOSITS_THRESHOLD_P) as usize
    }

    async fn mark_deposit_as_used(
        &self,
        deposit_id: DepositId,
        requested_on: OffsetDateTime,
        client_pubkey: PublicKeyUser,
        request_uuid: Uuid,
    ) -> Result<(), CredentialProxyError> {
        self.inner
            .storage
            .insert_deposit_usage(deposit_id, requested_on, client_pubkey, request_uuid)
            .await
    }

    async fn wait_for_deposit(
        &self,
        request_uuid: Uuid,
        requested_on: OffsetDateTime,
        client_pubkey: PublicKeyUser,
    ) -> Result<BufferedDeposit, CredentialProxyError> {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Some(buffered_deposit) = self.inner.unused_deposits.lock().await.pop() {
                // if the db call fails, we technically don't lose the deposit (we'll 'recover' it on restart)
                self.mark_deposit_as_used(
                    buffered_deposit.deposit_id,
                    requested_on,
                    client_pubkey,
                    request_uuid,
                )
                .await?;
                return Ok(buffered_deposit);
            } else {
                // make sure there's always a task working in the background in case deposits get used up too quickly
                self.maybe_refill_deposits()
            }
        }
    }

    pub(crate) async fn get_valid_deposit(
        &self,
        request_uuid: Uuid,
        requested_on: OffsetDateTime,
        client_pubkey: PublicKeyUser,
    ) -> Result<BufferedDeposit, CredentialProxyError> {
        let mut deposits_guard = self.inner.unused_deposits.lock().await;
        let deposits_available = deposits_guard.len();

        debug!("we have {deposits_available} unused deposits available");

        let maybe_deposit = deposits_guard.pop();
        drop(deposits_guard);

        if deposits_available < self.deposits_lower_threshold() {
            // if we're below threshold, start refill task
            self.maybe_refill_deposits()
        }

        match maybe_deposit {
            None => {
                warn!("we currently don't have any usable deposits! are we using them up faster than we request them?");

                // we have to wait until refill task has completed (either initiated by this or another fn call)
                self.wait_for_deposit(request_uuid, requested_on, client_pubkey)
                    .await
            }
            Some(buffered_deposit) => {
                self.mark_deposit_as_used(
                    buffered_deposit.deposit_id,
                    requested_on,
                    client_pubkey,
                    request_uuid,
                )
                .await?;
                Ok(buffered_deposit)
            }
        }
    }

    pub(crate) async fn wait_for_shutdown(&self) {
        let task_handle = self.inner.deposits_refill_task.take_task_join_handle();
        if let Some(task_handle) = task_handle {
            if !task_handle.is_finished() {
                info!("the deposit refill task is currently in progress - waiting for the current transaction to finish before concluding shutdown");
                let _ = task_handle.await;
            }
        }
    }
}

impl DepositsBufferInner {
    //
}
