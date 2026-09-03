// Copyright 2025 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::nym_api_helpers::{
    CachedEpoch, CachedImmutableEpochItem, ensure_sane_expiration_date, query_all_threshold_apis,
};
use crate::quorum_checker::QuorumState;
use crate::shared_state::nyxd_client::{ChainClient, EpochSource};
use crate::shared_state::required_deposit_cache::RequiredDepositCache;
use crate::storage::traits::GlobalEcashDataCache;
use nym_cache::CachedImmutableItems;
use nym_compact_ecash::Base58;
pub use nym_compact_ecash::VerificationKeyAuth;
pub use nym_compact_ecash::scheme::coin_indices_signatures::CoinIndexSignatureShare;
use nym_compact_ecash::scheme::coin_indices_signatures::aggregate_annotated_indices_signatures;
pub use nym_compact_ecash::scheme::expiration_date_signatures::ExpirationDateSignatureShare;
use nym_compact_ecash::scheme::expiration_date_signatures::aggregate_annotated_expiration_signatures;
use nym_credentials::ecash::utils::EcashTime;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
pub use nym_credentials::{IssuanceTicketBook, IssuedTicketBook};
pub use nym_credentials_interface::{TicketType, TicketTypeRepr};
use nym_http_api_client::{UserAgent, bin_info};
use nym_validator_client::EcashApiClient;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::{ContractVKShare, Epoch};
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, PagedDkgQueryClient};
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{info, warn};
use url::Url;

pub struct EcashState {
    pub required_deposit_cache: RequiredDepositCache,

    pub quorum_state: QuorumState,

    pub cached_epoch: RwLock<CachedEpoch>,

    pub master_verification_key: CachedImmutableEpochItem<VerificationKeyAuth>,

    pub threshold_values: CachedImmutableEpochItem<u64>,

    pub epoch_clients: CachedImmutableEpochItem<Vec<EcashApiClient>>,

    pub coin_index_signatures: CachedImmutableEpochItem<AggregatedCoinIndicesSignatures>,

    pub expiration_date_signatures:
        CachedImmutableItems<(EpochId, Date), AggregatedExpirationDateSignatures>,
}

fn construct_usable_ecash_api_clients(shares: Vec<ContractVKShare>) -> Vec<EcashApiClient> {
    let mut clients = Vec::with_capacity(shares.len());

    for share in shares {
        let owner = share.owner.clone();
        let epoch_id = share.epoch_id;

        match construct_ecash_api_client(share) {
            Ok(client) => clients.push(client),
            Err(err) => {
                warn!("ignoring the key share of {owner} for epoch {epoch_id}: {err}")
            }
        }
    }

    clients
}

fn construct_ecash_api_client(share: ContractVKShare) -> Result<EcashApiClient, EcashApiError> {
    if !share.verified {
        return Err(EcashApiError::UnverifiedShare);
    }

    let url_address = Url::parse(&share.announce_address)?;

    let api_client = nym_http_api_client::Client::builder(url_address)
        .map_err(|e| EcashApiError::ClientError(e.to_string()))?
        .with_timeout(Duration::from_secs(5))
        .with_user_agent(UserAgent::from(bin_info!()))
        .no_hickory_dns()
        .build()
        .map_err(|e| EcashApiError::ClientError(e.to_string()))?;

    Ok(EcashApiClient {
        api_client,
        verification_key: VerificationKeyAuth::try_from_bs58(&share.share)?,
        node_id: share.node_index,
        cosmos_address: share.owner.as_str().parse()?,
    })
}

/// What a caller has told us it is able to do, which decides whether issuing to it right now is
/// safe. Defaults to the least capable thing, because that is what a caller predating a given
/// field is.
#[derive(Debug, Default, Copy, Clone)]
pub struct CallerCapabilities {
    /// The caller states which epoch it means when fetching verification material, so a ceremony
    /// concluding between issuance and unblinding cannot leave it holding shares it can never use.
    pub epoch_aware: bool,
}

impl CallerCapabilities {
    pub fn epoch_aware(epoch_aware: bool) -> Self {
        CallerCapabilities { epoch_aware }
    }
}

impl EcashState {
    pub fn new(
        required_deposit_cache: RequiredDepositCache,
        quorum_state: QuorumState,
    ) -> EcashState {
        EcashState {
            required_deposit_cache,
            quorum_state,
            cached_epoch: Default::default(),
            master_verification_key: Default::default(),
            threshold_values: Default::default(),
            epoch_clients: Default::default(),
            coin_index_signatures: Default::default(),
            expiration_date_signatures: Default::default(),
        }
    }

    /// Refuse to *issue* to a caller that cannot say which epoch it means, while a ceremony runs.
    ///
    /// A book issued mid-ceremony belongs to the epoch still in service, which is not the chain's
    /// current one. A caller that omits the epoch when it later fetches verification material is
    /// served the current one, and once the ceremony concludes that no longer matches - so it can
    /// never unblind what it just paid for, and the failure is permanent because the epoch its
    /// shares were signed under never changes.
    ///
    /// Such a caller waits instead, which is recoverable and is exactly what it did before
    /// mid-ceremony issuance existed. Deliberately separate from
    /// [`Self::ensure_credentials_issuable`]: that one asks whether ecash works at all, and gates
    /// reads as well, which issue nothing and cannot strand anybody.
    pub async fn ensure_issuable_to_caller(
        &self,
        client: &impl EpochSource,
        caller: CallerCapabilities,
    ) -> Result<(), CredentialProxyError> {
        if caller.epoch_aware {
            return Ok(());
        }

        if !self.current_epoch(client).await?.state.is_final() {
            return Err(CredentialProxyError::CallerCannotIssueMidCeremony);
        }

        Ok(())
    }

    pub async fn ensure_credentials_issuable(
        &self,
        client: &impl EpochSource,
    ) -> Result<(), CredentialProxyError> {
        let epoch = self.current_epoch(client).await?;

        // a ceremony running no longer stops issuance: signers keep issuing under the epoch it is
        // replacing, so the only genuine refusal is when no epoch has ever concluded and there is
        // nothing to issue under at all
        if epoch.state.is_final() || epoch.epoch_id > 0 {
            Ok(())
        } else if let Some(final_timestamp) = epoch.final_timestamp_secs() {
            // SAFETY: the timestamp values in our DKG contract should be valid timestamps,
            // otherwise it means the chain is seriously misbehaving
            #[allow(clippy::unwrap_used)]
            let finish_dt = OffsetDateTime::from_unix_timestamp(final_timestamp as i64).unwrap();

            Err(CredentialProxyError::CredentialsNotYetIssuable {
                availability: finish_dt,
            })
        } else if epoch.state.is_waiting_initialisation() {
            Err(CredentialProxyError::UninitialisedDkg)
        } else {
            Err(CredentialProxyError::UnknownEcashFailure)
        }
    }

    pub async fn deposit_amount(&self, client: &ChainClient) -> Result<Coin, CredentialProxyError> {
        self.required_deposit_cache.get_or_update(client).await
    }

    /// Whether the ceremony for `epoch_id` has concluded, so its set of signers is settled.
    async fn ceremony_concluded(
        &self,
        client: &impl EpochSource,
        epoch_id: EpochId,
    ) -> Result<bool, CredentialProxyError> {
        Ok(self
            .current_epoch(client)
            .await?
            .is_ceremony_concluded(epoch_id))
    }

    /// The signers registered for `epoch_id`, skipping any whose share cannot be used.
    ///
    /// A single share that was never verified - or that carries an announce address the DKG
    /// contract never validated - must not deny the caller every *other* signer of that epoch.
    async fn registered_ecash_clients(
        &self,
        client: &ChainClient,
        epoch_id: EpochId,
    ) -> Result<Vec<EcashApiClient>, CredentialProxyError> {
        Ok(construct_usable_ecash_api_clients(
            client
                .query_chain()
                .await
                .get_all_verification_key_shares(epoch_id)
                .await?,
        ))
    }

    pub async fn ecash_clients(
        &self,
        client: &ChainClient,
        epoch_id: EpochId,
    ) -> Result<Vec<EcashApiClient>, CredentialProxyError> {
        // the moment a ceremony starts, the epoch id increments and the new epoch has no
        // verified shares yet. this cache has no expiry, so answering from it then would
        // remember an empty signer set for the life of the process - and this proxy would
        // keep failing to fan out long after the ceremony finished.
        if !self.ceremony_concluded(client, epoch_id).await? {
            return self.registered_ecash_clients(client, epoch_id).await;
        }

        self.epoch_clients
            .get_or_init(epoch_id, || async {
                self.registered_ecash_clients(client, epoch_id).await
            })
            .await
            .map(|guard| guard.clone())
    }

    pub async fn current_epoch(
        &self,
        client: &impl EpochSource,
    ) -> Result<Epoch, CredentialProxyError> {
        let read_guard = self.cached_epoch.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.current_epoch);
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.cached_epoch.write().await;
        let epoch = client.current_epoch().await?;

        write_guard.update(epoch);
        Ok(epoch)
    }

    /// The epoch signers are issuing under: the most recent whose ceremony has concluded. While a
    /// ceremony runs that is the epoch before the current one, whose keys exist and whose
    /// credentials are still circulating.
    ///
    /// Everything about one request has to agree on this - the signers asked, the threshold, the
    /// auxiliary data returned alongside the shares, and the epoch stated on each request - so it
    /// is resolved once per request and threaded through.
    pub async fn issuable_epoch_id(
        &self,
        client: &impl EpochSource,
    ) -> Result<EpochId, CredentialProxyError> {
        let epoch = self.current_epoch(client).await?;

        if epoch.state.is_final() {
            return Ok(epoch.epoch_id);
        }

        epoch
            .epoch_id
            .checked_sub(1)
            .ok_or(CredentialProxyError::UninitialisedDkg)
    }

    pub async fn master_verification_key<S>(
        &self,
        client: &ChainClient,
        storage: &S,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>, CredentialProxyError>
    where
        S: GlobalEcashDataCache,
    {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.issuable_epoch_id(client).await?,
        };

        self.master_verification_key
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(stored) = storage.get_master_verification_key(epoch_id).await? {
                    return Ok(stored.key);
                }

                info!("attempting to establish master verification key for epoch {epoch_id}...");

                // 2. perform actual aggregation
                let all_apis = self.ecash_clients(client, epoch_id).await?;
                let threshold = self.ecash_threshold(client, epoch_id).await?;

                if all_apis.len() < threshold as usize {
                    return Err(CredentialProxyError::InsufficientNumberOfSigners {
                        threshold,
                        available: all_apis.len(),
                    });
                }

                let master_key = nym_credentials::aggregate_verification_keys(&all_apis)?;

                let epoch = EpochVerificationKey {
                    epoch_id,
                    key: master_key,
                };

                // 3. save the key in the storage for when we reboot
                storage.insert_master_verification_key(&epoch).await?;

                Ok(epoch.key)
            })
            .await
    }

    pub async fn master_coin_index_signatures<S>(
        &self,
        client: &ChainClient,
        storage: &S,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, AggregatedCoinIndicesSignatures>, CredentialProxyError>
    where
        S: GlobalEcashDataCache,
    {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.issuable_epoch_id(client).await?,
        };

        self.coin_index_signatures
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(master_sigs) =
                    storage.get_master_coin_index_signatures(epoch_id).await?
                {
                    return Ok(master_sigs);
                }

                info!(
                    "attempting to establish master coin index signatures for epoch {epoch_id}..."
                );

                // 2. go around APIs and attempt to aggregate the data
                let master_vk = self
                    .master_verification_key(client, storage, Some(epoch_id))
                    .await?;
                let all_apis = self.ecash_clients(client, epoch_id).await?;
                let threshold = self.ecash_threshold(client, epoch_id).await?;

                let get_partial_signatures = |api: EcashApiClient| async {
                    // move the api into the closure
                    let api = api;
                    let node_index = api.node_id;
                    let partial_vk = api.verification_key;

                    let partial = api
                        .api_client
                        .partial_coin_indices_signatures(Some(epoch_id))
                        .await?
                        .signatures;
                    Ok(CoinIndexSignatureShare {
                        index: node_index,
                        key: partial_vk,
                        signatures: partial,
                    })
                };

                let shares =
                    query_all_threshold_apis(all_apis, threshold, get_partial_signatures).await?;

                let aggregated = aggregate_annotated_indices_signatures(
                    nym_credentials_interface::ecash_parameters(),
                    &master_vk,
                    &shares,
                )?;

                let sigs = AggregatedCoinIndicesSignatures {
                    epoch_id,
                    signatures: aggregated,
                };

                // 3. save the signatures in the storage for when we reboot
                storage.insert_master_coin_index_signatures(&sigs).await?;

                Ok(sigs)
            })
            .await
    }

    pub async fn master_expiration_date_signatures<S>(
        &self,
        client: &ChainClient,
        storage: &S,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<'_, AggregatedExpirationDateSignatures>, CredentialProxyError>
    where
        S: GlobalEcashDataCache,
    {
        self
            .expiration_date_signatures
            .get_or_init((epoch_id, expiration_date), || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(master_sigs) = storage
                    .get_master_expiration_date_signatures(expiration_date, epoch_id)
                    .await?
                {
                    return Ok(master_sigs);
                }


                info!(
                    "attempting to establish master expiration date signatures for {expiration_date} and epoch {epoch_id}..."
                );

                // 3. go around APIs and attempt to aggregate the data
                let master_vk = self.master_verification_key(client, storage, Some(epoch_id)).await?;
                let all_apis = self.ecash_clients(client, epoch_id).await?;
                let threshold = self.ecash_threshold(client, epoch_id).await?;

                let get_partial_signatures = |api: EcashApiClient| async {
                    // move the api into the closure
                    let api = api;
                    let node_index = api.node_id;
                    let partial_vk = api.verification_key;

                    let partial = api
                        .api_client
                        .partial_expiration_date_signatures(Some(expiration_date), Some(epoch_id))
                        .await?
                        .signatures;
                    Ok(ExpirationDateSignatureShare {
                        index: node_index,
                        key: partial_vk,
                        signatures: partial,
                    })
                };

                let shares =
                    query_all_threshold_apis(all_apis, threshold, get_partial_signatures)
                        .await?;

                let aggregated = aggregate_annotated_expiration_signatures(
                    &master_vk,
                    expiration_date.ecash_unix_timestamp(),
                    &shares,
                )?;

                let sigs = AggregatedExpirationDateSignatures {
                    epoch_id,
                    expiration_date,
                    signatures: aggregated,
                };

                // 4. save the signatures in the storage for when we reboot
                storage
                    .insert_master_expiration_date_signatures(&sigs)
                    .await?;

                Ok(sigs)
            })
            .await
    }

    pub async fn ecash_threshold(
        &self,
        client: &ChainClient,
        epoch_id: EpochId,
    ) -> Result<u64, CredentialProxyError> {
        self.threshold_values
            .get_or_init(epoch_id, || async {
                if let Some(threshold) = client
                    .query_chain()
                    .await
                    .get_epoch_threshold(epoch_id)
                    .await?
                {
                    Ok(threshold)
                } else {
                    Err(CredentialProxyError::UnavailableThreshold { epoch_id })
                }
            })
            .await
            .map(|t| *t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Timestamp;
    use nym_validator_client::nyxd::contract_traits::dkg_query_client::EpochState;

    /// The chain frozen at one point of one epoch.
    struct FixedEpoch(Epoch);

    impl EpochSource for FixedEpoch {
        async fn current_epoch(&self) -> Result<Epoch, CredentialProxyError> {
            Ok(self.0)
        }
    }

    fn state() -> EcashState {
        EcashState::new(RequiredDepositCache::default(), QuorumState::fixed(true))
    }

    /// `epoch_id`'s ceremony is running, so the keys in service are the previous epoch's.
    fn mid_ceremony(epoch_id: EpochId) -> FixedEpoch {
        FixedEpoch(Epoch {
            state: EpochState::DealingExchange { resharing: true },
            epoch_id,
            ..Default::default()
        })
    }

    /// `epoch_id`'s ceremony concluded and its keys are the ones in service.
    fn concluded(epoch_id: EpochId) -> FixedEpoch {
        FixedEpoch(Epoch {
            state: EpochState::InProgress,
            epoch_id,
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn epoch_aware_caller_may_issue_mid_ceremony() {
        assert!(
            state()
                .ensure_issuable_to_caller(&mid_ceremony(5), CallerCapabilities::epoch_aware(true))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn caller_that_cannot_state_the_epoch_is_refused_mid_ceremony() {
        let err = state()
            .ensure_issuable_to_caller(&mid_ceremony(5), CallerCapabilities::default())
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            CredentialProxyError::CallerCannotIssueMidCeremony
        ));
    }

    #[tokio::test]
    async fn caller_that_cannot_state_the_epoch_may_issue_between_ceremonies() {
        assert!(
            state()
                .ensure_issuable_to_caller(&concluded(5), CallerCapabilities::default())
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn a_running_ceremony_does_not_stop_issuance_once_an_epoch_has_concluded() {
        assert!(
            state()
                .ensure_credentials_issuable(&mid_ceremony(5))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn issuance_is_refused_until_the_first_ceremony_concludes() {
        let deadline = 1_700_000_000;
        let first_ceremony = FixedEpoch(Epoch {
            state: EpochState::PublicKeySubmission { resharing: false },
            epoch_id: 0,
            deadline: Some(Timestamp::from_seconds(deadline)),
            ..Default::default()
        });

        let err = state()
            .ensure_credentials_issuable(&first_ceremony)
            .await
            .unwrap_err();

        let CredentialProxyError::CredentialsNotYetIssuable { availability } = err else {
            panic!("expected the refusal to say when issuance becomes available, got: {err}")
        };
        // the caller is pointed at the end of the whole ceremony, not of its current state
        assert!(availability > OffsetDateTime::from_unix_timestamp(deadline as i64).unwrap());
    }

    #[tokio::test]
    async fn issuance_is_refused_while_the_dkg_is_uninitialised() {
        let uninitialised = FixedEpoch(Epoch::default());
        assert!(uninitialised.0.state.is_waiting_initialisation());

        let err = state()
            .ensure_credentials_issuable(&uninitialised)
            .await
            .unwrap_err();

        assert!(matches!(err, CredentialProxyError::UninitialisedDkg));
    }

    #[tokio::test]
    async fn issuable_epoch_is_the_current_one_between_ceremonies() {
        assert_eq!(5, state().issuable_epoch_id(&concluded(5)).await.unwrap());
    }

    #[tokio::test]
    async fn issuable_epoch_is_the_one_being_replaced_mid_ceremony() {
        assert_eq!(
            4,
            state().issuable_epoch_id(&mid_ceremony(5)).await.unwrap()
        );
    }

    #[tokio::test]
    async fn no_epoch_is_issuable_during_the_very_first_ceremony() {
        let err = state()
            .issuable_epoch_id(&mid_ceremony(0))
            .await
            .unwrap_err();

        assert!(matches!(err, CredentialProxyError::UninitialisedDkg));
    }
}
