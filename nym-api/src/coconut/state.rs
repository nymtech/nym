// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::client::Client as LocalClient;
use crate::coconut::comm::APICommunicationChannel;
use crate::coconut::deposit::validate_deposit_tx;
use crate::coconut::error::{CoconutError, Result};
use crate::coconut::keys::KeyPair;
use crate::coconut::storage::CoconutStorageExt;
use crate::support::storage::NymApiStorage;
use nym_api_requests::coconut::helpers::issued_credential_plaintext;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_coconut::{BlindedSignature, VerificationKey};
use nym_coconut_dkg_common::types::EpochId;
use nym_crypto::asymmetric::identity;
use nym_validator_client::nyxd::{AccountId, Hash, TxResponse};
use rand::rngs::OsRng;
use rand::RngCore;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tokio::sync::{OnceCell, RwLock};

pub use nym_credentials::coconut::bandwidth::bandwidth_credential_params;

pub struct State {
    pub(crate) client: Arc<dyn LocalClient + Send + Sync>,
    pub(crate) bandwidth_contract_admin: OnceCell<Option<AccountId>>,
    pub(crate) mix_denom: String,
    pub(crate) coconut_keypair: KeyPair,
    pub(crate) identity_keypair: identity::KeyPair,
    pub(crate) comm_channel: Arc<dyn APICommunicationChannel + Send + Sync>,
    pub(crate) storage: NymApiStorage,
    pub(crate) freepass_nonce: Arc<RwLock<[u8; 16]>>,
    pub(crate) authorised_freepass_requester: Arc<RwLock<AuthorisedFreepassRequester>>,
}

const FREEPASS_REQUESTER_TTL: Duration = Duration::hours(1);
const AUTHORISED_FREEPASS_REQUESTER_ENDPOINT: &str =
    "https://nymtech.net/.wellknown/authorised-freepass-requester.txt";

pub struct AuthorisedFreepassRequester {
    address: Option<AccountId>,
    refreshed_at: OffsetDateTime,
}

impl Default for AuthorisedFreepassRequester {
    fn default() -> Self {
        AuthorisedFreepassRequester {
            address: None,
            refreshed_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}

impl State {
    pub(crate) fn new<C, D>(
        client: C,
        mix_denom: String,
        identity_keypair: identity::KeyPair,
        key_pair: KeyPair,
        comm_channel: D,
        storage: NymApiStorage,
    ) -> Self
    where
        C: LocalClient + Send + Sync + 'static,
        D: APICommunicationChannel + Send + Sync + 'static,
    {
        let client = Arc::new(client);
        let comm_channel = Arc::new(comm_channel);

        let mut nonce = [0u8; 16];
        OsRng.fill_bytes(&mut nonce);

        Self {
            client,
            bandwidth_contract_admin: OnceCell::new(),
            mix_denom,
            coconut_keypair: key_pair,
            identity_keypair,
            comm_channel,
            storage,
            freepass_nonce: Arc::new(RwLock::new(nonce)),
            authorised_freepass_requester: Arc::new(Default::default()),
        }
    }

    /// Check if this nym-api has already issued a credential for the provided deposit hash.
    /// If so, return it.
    pub async fn already_issued(&self, tx_hash: Hash) -> Result<Option<BlindedSignature>> {
        self.storage
            .get_issued_bandwidth_credential_by_hash(&tx_hash.to_string())
            .await?
            .map(|cred| cred.try_into())
            .transpose()
    }

    pub async fn get_transaction(&self, tx_hash: Hash) -> Result<TxResponse> {
        self.client.get_tx(tx_hash).await
    }

    pub async fn get_bandwidth_contract_admin(&self) -> Result<&Option<AccountId>> {
        self.bandwidth_contract_admin
            .get_or_try_init(|| async { self.client.bandwidth_contract_admin().await })
            .await
    }

    async fn try_get_authorised_freepass_requester(&self) -> Result<AccountId> {
        let address = reqwest::Client::builder()
            .user_agent(format!(
                "nym-api / {} identity: {}",
                env!("CARGO_PKG_VERSION"),
                self.identity_keypair.public_key().to_base58_string()
            ))
            .build()?
            .get(AUTHORISED_FREEPASS_REQUESTER_ENDPOINT)
            .send()
            .await?
            .text()
            .await?;
        let trimmed = address.trim();

        address.parse().map_err(
            |_| CoconutError::MalformedAuthorisedFreepassRequesterAddress {
                address: trimmed.to_string(),
            },
        )
    }

    pub async fn get_authorised_freepass_requester(&self) -> Option<AccountId> {
        {
            let cached = self.authorised_freepass_requester.read().await;

            // the entry hasn't expired
            if cached.refreshed_at + FREEPASS_REQUESTER_TTL >= OffsetDateTime::now_utc() {
                if let Some(cached_address) = cached.address.as_ref() {
                    return Some(cached_address.clone());
                }
            }
        }

        // refresh cache
        let mut cache = self.authorised_freepass_requester.write().await;

        // whatever happens, update refresh time
        cache.refreshed_at = OffsetDateTime::now_utc();

        let refreshed = match self.try_get_authorised_freepass_requester().await {
            Ok(upstream) => upstream,
            Err(err) => {
                warn!("failed to obtain authorised freepass requester address: {err}");
                return None;
            }
        };

        cache.address = Some(refreshed.clone());
        Some(refreshed)
    }

    pub async fn validate_request(
        &self,
        request: &BlindSignRequestBody,
        tx: TxResponse,
    ) -> Result<()> {
        validate_deposit_tx(request, tx).await
    }

    pub(crate) async fn sign_and_store_credential(
        &self,
        current_epoch: EpochId,
        request_body: BlindSignRequestBody,
        blinded_signature: &BlindedSignature,
    ) -> Result<i64> {
        let encoded_commitments = request_body.encode_commitments();

        let plaintext = issued_credential_plaintext(
            current_epoch as u32,
            request_body.tx_hash,
            blinded_signature,
            &encoded_commitments,
            &request_body.public_attributes_plain,
        );

        let signature = self.identity_keypair.private_key().sign(plaintext);

        // note: we have a UNIQUE constraint on the tx_hash column of the credential
        // and so if the api is processing request for the same hash at the same time,
        // only one of them will be successfully inserted to the database
        let credential_id = self
            .storage
            .store_issued_credential(
                current_epoch as u32,
                request_body.tx_hash,
                blinded_signature,
                signature,
                encoded_commitments,
                request_body.public_attributes_plain,
            )
            .await?;

        Ok(credential_id)
    }

    pub async fn store_issued_credential(
        &self,
        request_body: BlindSignRequestBody,
        blinded_signature: &BlindedSignature,
    ) -> Result<()> {
        let current_epoch = self.comm_channel.current_epoch().await?;

        // note: we have a UNIQUE constraint on the tx_hash column of the credential
        // and so if the api is processing request for the same hash at the same time,
        // only one of them will be successfully inserted to the database
        let credential_id = self
            .sign_and_store_credential(current_epoch, request_body, blinded_signature)
            .await?;
        self.storage
            .update_epoch_credentials_entry(current_epoch, credential_id)
            .await?;
        debug!("the stored credential has id {credential_id}");

        Ok(())
    }

    pub async fn verification_key(&self, epoch_id: EpochId) -> Result<VerificationKey> {
        self.comm_channel
            .aggregated_verification_key(epoch_id)
            .await
    }
}
