// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{path::Path, sync::Arc, time::Duration};

use async_trait::async_trait;
use nym_bandwidth_controller::{
    CredentialFetcher, CredentialFetcherError, CredentialPublicDataFetcher, NymCredential,
    TicketType,
};
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    IssuanceTicketBook, IssuedTicketBook, obtain_aggregate_wallet,
};
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::{Date, OffsetDateTime, ecash_default_expiration_date};
use nym_validator_client::{
    nym_api::EpochId,
    nyxd::{
        Coin, CosmWasmClient,
        contract_traits::{
            DkgQueryClient, EcashQueryClient, EcashSigningClient, dkg_query_client::EpochState,
        },
        cosmwasm_client::ContractResponseData,
    },
    signing::signer::OfflineSigner,
};
use rand::rngs::OsRng;
use tracing::{debug, error, info, warn};
use zeroize::Zeroizing;

use crate::{
    EcashApiClientsCache, NyxdGlobalDataFetcher, error::NyxdFetcherError,
    storage::PendingCredentialRequestsStorage,
};

/// Obtains ticketbooks by depositing on-chain and aggregating wallet signatures from the ecash
/// APIs, and recovers ticketbooks from deposits whose issuance never completed. Deposits are
/// serialised via `deposit_lock` so concurrent fetches don't race the account sequence number.
///
// Batching them is TODO, it requires extensive changes to the BC
pub struct NyxdCredentialFetcher<C> {
    client: Arc<C>,
    client_id: Zeroizing<Vec<u8>>,
    pending_storage: PendingCredentialRequestsStorage,
    ecash_api_clients: Arc<EcashApiClientsCache>,
    public_data_fetcher: NyxdGlobalDataFetcher<C>,
    // serialises on-chain deposits so concurrent fetches can't race the account sequence number.
    deposit_lock: tokio::sync::Mutex<()>,
}

impl<C> NyxdCredentialFetcher<C>
where
    C: DkgQueryClient,
{
    /// Creates a fetcher whose pending-request recovery store lives at `db_path`.
    pub async fn new(
        client: Arc<C>,
        db_path: impl AsRef<Path>,
        client_id: Zeroizing<Vec<u8>>,
    ) -> Result<Self, NyxdFetcherError> {
        let pending_storage = PendingCredentialRequestsStorage::init(db_path).await?;
        let ecash_api_clients = Arc::new(EcashApiClientsCache::new());
        let public_data_fetcher = NyxdGlobalDataFetcher::new_with_ecash_clients(
            client.clone(),
            ecash_api_clients.clone(),
        );

        Ok(NyxdCredentialFetcher {
            client,
            client_id,
            ecash_api_clients,
            pending_storage,
            public_data_fetcher,
            deposit_lock: tokio::sync::Mutex::new(()),
        })
    }

    async fn block_until_ecash_is_available(&self) -> Result<(), NyxdFetcherError> {
        loop {
            let epoch = self.client.get_current_epoch().await?;
            let current_timestamp_secs = OffsetDateTime::now_utc().unix_timestamp() as u64;

            if epoch.state.is_final() {
                break;
            } else if let Some(final_timestamp) = epoch.final_timestamp_secs() {
                // Use 1 additional second to not start the next iteration immediately and spam get_current_epoch queries
                let secs_until_final = final_timestamp.saturating_sub(current_timestamp_secs) + 1;
                info!(
                    "Approximately {secs_until_final} seconds until coconut is available. Sleeping until then. You can safely kill the process at any moment."
                );
                tokio::time::sleep(Duration::from_secs(secs_until_final)).await;
            } else if matches!(epoch.state, EpochState::WaitingInitialisation) {
                info!(
                    "dkg hasn't been initialised yet and it is not known when it will be. Going to check again later"
                );
                tokio::time::sleep(Duration::from_secs(60 * 5)).await;
            } else {
                // this should never be the case since the only case where final timestamp is unknown is when it's waiting for initialisation,
                // but let's guard ourselves against future changes
                info!("it is unknown when ecash will become available. Going to check again later");
                tokio::time::sleep(Duration::from_secs(60 * 5)).await;
            }
        }

        Ok(())
    }

    async fn recover_deposits(
        &self,
        ticketbook_type: TicketType,
    ) -> Result<Vec<NymCredential>, NyxdFetcherError> {
        info!("checking for any incomplete previous issuance attempts...");

        let incomplete = self
            .pending_storage
            .get_pending_ticketbooks()
            .await?
            .into_iter()
            .filter(|ticket_book| {
                ticket_book.pending_ticketbook.ticketbook_type() == ticketbook_type
            })
            .collect::<Vec<_>>();
        info!(
            "we recovered {} incomplete ticketbook issuances",
            incomplete.len()
        );

        let mut recovered_books = Vec::new();
        for issuance in incomplete {
            let deposit = issuance.pending_ticketbook.deposit_id();
            if issuance.pending_ticketbook.expired() {
                warn!(
                    "ticketbook data associated with deposit {deposit} has expired. if you haven't contacted more than 1/3 of signers. it could still be recoverable (but out of scope of this library)"
                );
                continue;
            }

            if issuance.pending_ticketbook.check_expiration_date() {
                warn!(
                    "deposit {deposit} was made with a different expiration date, its validity will be shorter than the max one"
                );
            }

            match self.obtain_ticketbook(&issuance.pending_ticketbook).await {
                Err(err) => error!("could not recover deposit {deposit} due to: {err}"),
                Ok(issued) => {
                    recovered_books.push(NymCredential::Ticketbook(Box::new(issued)));
                    info!("managed to recover deposit {deposit}! ");
                    if let Err(e) = self
                        .pending_storage
                        .remove_pending_ticketbook(issuance.pending_id)
                        .await
                    {
                        warn!("Failed to remove the data from pending storage : {e}");
                    };
                }
            }
        }

        Ok(recovered_books)
    }

    /// Obtains an issued ticketbook by aggregating the partial wallet signatures from the ecash APIs.
    ///
    /// This performs only the network/cryptographic work and returns the resulting [`IssuedTicketBook`];
    /// persisting it (and the global materials it needs) is the responsibility of the
    /// [`crate::BandwidthController`] via `import_ticketbook`.
    async fn obtain_ticketbook(
        &self,
        issuance_data: &IssuanceTicketBook,
    ) -> Result<IssuedTicketBook, NyxdFetcherError> {
        let epoch_id = self.client.get_current_epoch().await?.epoch_id;
        let threshold = self
            .client
            .get_current_epoch_threshold()
            .await?
            .ok_or(NyxdFetcherError::NoThreshold)?;

        let apis = self.ecash_api_clients.get(&*self.client, epoch_id).await?;

        info!("Querying wallet signatures");
        let wallet = obtain_aggregate_wallet(issuance_data, &apis, threshold).await?;
        info!("managed to obtain sufficient number of partial signatures!");

        Ok(issuance_data.to_issued_ticketbook(wallet, epoch_id))
    }
}

impl<C> NyxdCredentialFetcher<C>
where
    C: DkgQueryClient + EcashSigningClient + EcashQueryClient,
{
    async fn make_deposit(
        &self,
        expiration: Date,
        ticketbook_type: TicketType,
    ) -> Result<IssuanceTicketBook, NyxdFetcherError> {
        let mut rng = OsRng;
        let signing_key = ed25519::PrivateKey::new(&mut rng);

        let deposit_amount = self.client.get_default_deposit_amount().await?;
        info!("we'll need to deposit {deposit_amount} to obtain the ticketbook");

        // serialise deposits: overlapping fetches would otherwise race the account sequence number.
        // `make_ticketbook_deposit` broadcasts-and-waits-for-commit, so by the time the tx returns
        // the sequence has advanced on-chain - holding the lock across it is enough.
        let _deposit_guard = self.deposit_lock.lock().await;
        let result = self
            .client
            .make_ticketbook_deposit(
                signing_key.public_key().to_base58_string(),
                deposit_amount.into(),
                None,
            )
            .await?;

        let deposit_id = result.parse_singleton_u32_contract_data()?;

        info!("our ticketbook deposit has been stored under id {deposit_id}");

        Ok(IssuanceTicketBook::new_with_expiration(
            deposit_id,
            &self.client_id,
            signing_key,
            ticketbook_type,
            expiration,
        ))
    }
}

impl<C> NyxdCredentialFetcher<C>
where
    C: DkgQueryClient + EcashQueryClient + CosmWasmClient + OfflineSigner,
{
    // conservative per-deposit tx fee estimate used when checking the balance
    const DEPOSIT_TX_FEE_AMOUNT: u128 = 50_000;

    /// Returns whether the signer account holds enough to fund `nb_deposits` deposits (plus fees).
    /// A shortfall is reported as `Ok(false)` (and logged), not an error.
    pub async fn check_balance(&self, nb_deposits: u128) -> Result<bool, NyxdFetcherError> {
        // determine required deposits amount and funds and ensure we have enough
        // (plus a bit more for tx fees)
        let raw_deposit_cost = self.client.get_default_deposit_amount().await?;
        let per_deposit_cost_amount = raw_deposit_cost.amount.u128() + Self::DEPOSIT_TX_FEE_AMOUNT;
        let total_deposits_cost = Coin::new(
            per_deposit_cost_amount * nb_deposits,
            raw_deposit_cost.denom,
        );

        info!(
            "this will require {nb_deposits} deposits that will cost approximately {total_deposits_cost}"
        );

        let client_address = self.client.signer_addresses()[0].clone();

        let available_balance = self
            .client
            .get_balance(&client_address, total_deposits_cost.denom.clone())
            .await?
            .unwrap_or_else(|| Coin::new(0, &total_deposits_cost.denom));
        debug!("available_balance: {available_balance}");

        let sufficient_funds = available_balance.amount > total_deposits_cost.amount;

        if !sufficient_funds {
            warn!(
                "insufficient funds for obtaining desired amount of ticketbooks. available: {available_balance} required (approximately): {total_deposits_cost}"
            );
        }
        Ok(sufficient_funds)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> CredentialPublicDataFetcher for NyxdCredentialFetcher<C>
where
    C: DkgQueryClient,
{
    async fn fetch_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<EpochVerificationKey, CredentialFetcherError> {
        self.public_data_fetcher
            .fetch_master_verification_key(epoch_id)
            .await
    }

    async fn fetch_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
        self.public_data_fetcher
            .fetch_coin_index_signatures(epoch_id)
            .await
    }

    async fn fetch_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
        self.public_data_fetcher
            .fetch_expiration_date_signatures(expiration_date, epoch_id)
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> CredentialFetcher for NyxdCredentialFetcher<C>
where
    C: DkgQueryClient + EcashSigningClient + EcashQueryClient,
{
    async fn fetch_ticketbooks(
        &self,
        ticketbook_type: TicketType,
    ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
        self.block_until_ecash_is_available().await?;

        if let Ok(recovered_ticketbooks) = self.recover_deposits(ticketbook_type).await {
            if !recovered_ticketbooks.is_empty() {
                info!(
                    "managed to recover {} ticket books. no need to make fresh deposit",
                    recovered_ticketbooks.len()
                );
                return Ok(recovered_ticketbooks);
            }
        };

        let ticketbook_expiration = ecash_default_expiration_date();

        info!("Starting to deposit funds, don't kill the process");
        let issuance_data = self
            .make_deposit(ticketbook_expiration, ticketbook_type)
            .await?;
        info!("Deposit done");

        match self.obtain_ticketbook(&issuance_data).await {
            Ok(issued) => {
                info!("Succeeded adding a ticketbook of type '{ticketbook_type}'");
                Ok(vec![NymCredential::Ticketbook(Box::new(issued))])
            }
            Err(e) => {
                error!("failed to obtain credential. saving recovery data...");

                self.pending_storage
                    .insert_pending_ticketbook(&issuance_data)
                    .await
                    .inspect_err(|err| {
                        let deposit = issuance_data.deposit_id();
                        error!("could not save the recovery data for deposit {deposit}: {err}. the data will unfortunately get lost")
                    })
                    .map_err(NyxdFetcherError::from)?;

                Err(e.into())
            }
        }
    }

    async fn cleanup(&self) {
        self.pending_storage.close().await;
    }

    async fn reset(mut self) -> Result<(), CredentialFetcherError> {
        Ok(self
            .pending_storage
            .reset()
            .await
            .map_err(NyxdFetcherError::from)?)
    }
}

#[cfg(feature = "recovery")]
pub(crate) mod recovery {
    use super::*;

    /// Recover-only view over [`NyxdCredentialFetcher`]: its `CredentialFetcher` impl only recovers pending deposits (never makes new
    /// ones), so it works with a plain query client.
    pub struct NyxdRecoveryFetcher<C>(NyxdCredentialFetcher<C>);

    impl<C> NyxdRecoveryFetcher<C>
    where
        C: DkgQueryClient,
    {
        pub async fn new(
            client: Arc<C>,
            db_path: impl AsRef<Path>,
        ) -> Result<Self, NyxdFetcherError> {
            // recovery never deposits, so the client id is unused
            Ok(Self(
                NyxdCredentialFetcher::new(client, db_path, Zeroizing::new(Vec::new())).await?,
            ))
        }
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl<C> CredentialPublicDataFetcher for NyxdRecoveryFetcher<C>
    where
        C: DkgQueryClient,
    {
        async fn fetch_master_verification_key(
            &self,
            epoch_id: EpochId,
        ) -> Result<EpochVerificationKey, CredentialFetcherError> {
            self.0.fetch_master_verification_key(epoch_id).await
        }

        async fn fetch_coin_index_signatures(
            &self,
            epoch_id: EpochId,
        ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
            self.0.fetch_coin_index_signatures(epoch_id).await
        }

        async fn fetch_expiration_date_signatures(
            &self,
            expiration_date: Date,
            epoch_id: EpochId,
        ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
            self.0
                .fetch_expiration_date_signatures(expiration_date, epoch_id)
                .await
        }
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl<C> CredentialFetcher for NyxdRecoveryFetcher<C>
    where
        C: DkgQueryClient,
    {
        async fn fetch_ticketbooks(
            &self,
            ticketbook_type: TicketType,
        ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
            self.0.block_until_ecash_is_available().await?;

            let recovered_ticketbooks = self.0.recover_deposits(ticketbook_type).await?;
            info!(
                "managed to recover {} ticket books",
                recovered_ticketbooks.len()
            );
            Ok(recovered_ticketbooks)
        }

        async fn cleanup(&self) {
            self.0.pending_storage.close().await;
        }

        async fn reset(mut self) -> Result<(), CredentialFetcherError> {
            Ok(self
                .0
                .pending_storage
                .reset()
                .await
                .map_err(NyxdFetcherError::from)?)
        }
    }
}
