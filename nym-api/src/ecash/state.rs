// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::helpers::{accepted_vote_err, CoinIndexSignatureCache, ExpirationDateSignatureCache};
use crate::ecash::client::Client as LocalClient;
use crate::ecash::comm::APICommunicationChannel;
use crate::ecash::deposit::validate_deposit;
use crate::ecash::error::{CoconutError, Result};
use crate::ecash::keys::KeyPair;
use crate::ecash::storage::CoconutStorageExt;
use crate::support::storage::NymApiStorage;
use bloomfilter::Bloom;
use cw3::Status;
use nym_api_requests::coconut::helpers::issued_credential_plaintext;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::{
    scheme::coin_indices_signatures::CoinIndexSignature,
    scheme::expiration_date_signatures::{sign_expiration_date, ExpirationDateSignature},
    BlindedSignature, VerificationKeyAuth,
};
use nym_config::defaults::{BLOOM_BITMAP_SIZE, BLOOM_NUM_HASHES, BLOOM_SIP_KEYS};
use nym_credentials::{coconut::utils::cred_exp_date, CredentialSpendingData};
use nym_crypto::asymmetric::identity;
use nym_ecash_contract_common::deposit::{Deposit, DepositId};
use nym_ecash_contract_common::spend_credential::check_proposal;
use nym_validator_client::nyxd::cosmwasm_client::logs::find_proposal_id;
use nym_validator_client::nyxd::AccountId;
use rand::rngs::OsRng;
use rand::RngCore;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tokio::sync::{OnceCell, RwLock};

pub struct State {
    pub(crate) client: Arc<dyn LocalClient + Send + Sync>,
    pub(crate) bandwidth_contract_admin: OnceCell<Option<AccountId>>,
    pub(crate) ecash_keypair: KeyPair,
    pub(crate) identity_keypair: identity::KeyPair,
    pub(crate) comm_channel: Arc<dyn APICommunicationChannel + Send + Sync>,
    pub(crate) storage: NymApiStorage,
    coin_indices_sigs_cache: Arc<CoinIndexSignatureCache>,
    exp_date_sigs_cache: Arc<ExpirationDateSignatureCache>,
    pub(crate) freepass_nonce: Arc<RwLock<[u8; 16]>>,
    spent_credentials: Arc<RwLock<Bloom<String>>>,
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

        let bitmap = [0u8; (BLOOM_BITMAP_SIZE / 8) as usize];
        let bloom_filter =
            Bloom::from_existing(&bitmap, BLOOM_BITMAP_SIZE, BLOOM_NUM_HASHES, BLOOM_SIP_KEYS);
        Self {
            client,
            bandwidth_contract_admin: OnceCell::new(),
            ecash_keypair: key_pair,
            identity_keypair,
            comm_channel,
            storage,
            coin_indices_sigs_cache: Arc::new(CoinIndexSignatureCache::new()),
            exp_date_sigs_cache: Arc::new(ExpirationDateSignatureCache::new()),
            freepass_nonce: Arc::new(RwLock::new(nonce)),
            spent_credentials: Arc::new(RwLock::new(bloom_filter)),
            authorised_freepass_requester: Arc::new(Default::default()),
        }
    }

    /// Check if this nym-api has already issued a credential for the provided deposit id.
    /// If so, return it.
    pub async fn already_issued(&self, deposit_id: DepositId) -> Result<Option<BlindedSignature>> {
        self.storage
            .get_issued_bandwidth_credential_by_deposit_id(deposit_id)
            .await?
            .map(|cred| cred.try_into())
            .transpose()
    }

    pub async fn get_deposit(&self, deposit_id: DepositId) -> Result<Deposit> {
        self.client
            .get_deposit(deposit_id)
            .await?
            .deposit
            .ok_or(CoconutError::NonExistentDeposit { deposit_id })
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
        deposit: Deposit,
    ) -> Result<()> {
        validate_deposit(request, deposit).await
    }

    pub(crate) async fn accept_and_execute_proposal(
        &self,
        proposal_id: Option<u64>,
        serial_number_bs58: String,
    ) -> Result<()> {
        if let Some(id) = proposal_id {
            let proposal = self.client.get_proposal(id).await?;

            // Proposal description is the blinded serial number
            if serial_number_bs58 != proposal.description {
                return Err(CoconutError::IncorrectProposal {
                    reason: String::from("incorrect blinded serial number in description"),
                });
            }
            if !check_proposal(proposal.msgs) {
                return Err(CoconutError::IncorrectProposal {
                    reason: String::from("action is not to spend_credential"),
                });
            }
            let client = self.client.clone();
            tokio::spawn(async move {
                let ret = client.vote_proposal(id, true, None).await;
                //SW NOTE: What to do if this fails
                if let Err(e) = accepted_vote_err(ret) {
                    log::debug!("failed to vote on proposal {id} - {e}");
                }

                if let Ok(proposal) = client.get_proposal(id).await {
                    if proposal.status == Status::Passed {
                        //SW NOTE: What to do if this fails
                        if let Err(e) = client.execute_proposal(id).await {
                            log::debug!("failed to execute proposal {id} - {e}");
                        }
                    }
                }
            });
        }
        Ok(())
    }

    pub(crate) async fn refuse_proposal(&self, proposal_id: Option<u64>) {
        if let Some(id) = proposal_id {
            let client = self.client.clone();
            tokio::spawn(async move {
                //whatever is in the proposal, we can refuse it anyway
                if let Err(e) = client.vote_proposal(id, false, None).await {
                    log::debug!("failed to refuse proposal {id} - {e}")
                }
            });
        }
    }

    pub(crate) async fn refuse_proposal_online(&self, proposal_id: Option<u64>) {
        if let Some(id) = proposal_id {
            //whatever is in the proposal, we can refuse it anyway
            if let Err(e) = self.client.vote_proposal(id, false, None).await {
                log::debug!("failed to refuse proposal {id} - {e}")
            }
        }
    }

    pub(crate) async fn blacklist(&self, public_key: String) {
        let client = self.client.clone();
        tokio::spawn(async move {
            //SW TODO error handling with one log at the end
            let response = client.propose_for_blacklist(public_key.clone()).await?;
            let proposal_id = find_proposal_id(&response.logs)?;

            let proposal = client.get_proposal(proposal_id).await?;
            if proposal.status == Status::Open {
                if public_key != proposal.description {
                    return Err(CoconutError::IncorrectProposal {
                        reason: String::from("incorrect publickey in description"),
                    });
                }
                let ret = client.vote_proposal(proposal_id, true, None).await;

                accepted_vote_err(ret)?;

                if let Ok(proposal) = client.get_proposal(proposal_id).await {
                    if proposal.status == Status::Passed {
                        client.execute_proposal(proposal_id).await?
                    }
                }
            }
            Ok(())
        });
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
            request_body.deposit_id,
            blinded_signature,
            &encoded_commitments,
            request_body.expiration_date,
        );

        let signature = self.identity_keypair.private_key().sign(plaintext);

        // note: we have a UNIQUE constraint on the tx_hash column of the credential
        // and so if the api is processing request for the same hash at the same time,
        // only one of them will be successfully inserted to the database
        let credential_id = self
            .storage
            .store_issued_credential(
                current_epoch as u32,
                request_body.deposit_id,
                blinded_signature,
                signature,
                encoded_commitments,
                request_body.expiration_date,
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

    pub async fn verification_key(&self, epoch_id: EpochId) -> Result<VerificationKeyAuth> {
        self.comm_channel
            .aggregated_verification_key(epoch_id)
            .await
    }

    pub async fn store_credential(
        &self,
        credential: &CredentialSpendingData,
        gateway_addr: &AccountId,
        proposal_id: u64,
    ) -> Result<()> {
        self.storage
            .insert_credential(
                credential,
                credential.serial_number_b58(),
                gateway_addr,
                proposal_id,
            )
            .await
            .map_err(|err| err.into())
    }

    pub async fn get_credential_by_sn(
        &self,
        serial_number_bs58: String,
    ) -> Result<Option<CredentialSpendingData>> {
        self.storage
            .get_credential(serial_number_bs58)
            .await
            .map_err(|err| err.into())
    }

    pub async fn get_coin_indices_signatures(&self) -> Result<Vec<CoinIndexSignature>> {
        let current_epoch = self.comm_channel.current_epoch().await?;
        match self
            .coin_indices_sigs_cache
            .get_signatures(current_epoch)
            .await
        {
            Some(signatures) => Ok(signatures),
            None => {
                let verification_key = self.verification_key(current_epoch).await?;
                let maybe_keypair_guard = self.ecash_keypair.get().await;
                let Some(keypair_guard) = maybe_keypair_guard.as_ref() else {
                    return Err(CoconutError::KeyPairNotDerivedYet);
                };
                let Some(signing_key) = keypair_guard.as_ref() else {
                    return Err(CoconutError::KeyPairNotDerivedYet);
                };
                self.coin_indices_sigs_cache
                    .refresh_signatures(
                        current_epoch,
                        &verification_key,
                        signing_key.keys.secret_key(),
                    )
                    .await
            }
        }
    }

    pub async fn get_exp_date_signatures(&self) -> Result<Vec<ExpirationDateSignature>> {
        let current_epoch = self.comm_channel.current_epoch().await?;
        let expiration_ts = cred_exp_date().unix_timestamp() as u64;
        match self
            .exp_date_sigs_cache
            .get_signatures(current_epoch, expiration_ts)
            .await
        {
            Some(signatures) => Ok(signatures),
            None => {
                let maybe_keypair_guard = self.ecash_keypair.get().await;
                let Some(keypair_guard) = maybe_keypair_guard.as_ref() else {
                    return Err(CoconutError::KeyPairNotDerivedYet);
                };
                let Some(signing_key) = keypair_guard.as_ref() else {
                    return Err(CoconutError::KeyPairNotDerivedYet);
                };
                self.exp_date_sigs_cache
                    .refresh_signatures(current_epoch, expiration_ts, signing_key.keys.secret_key())
                    .await
            }
        }
    }

    //this one gives the signatures for a particular day. No cache because it's only gonna be used for recovery attempt and freepasses
    pub async fn get_exp_date_signatures_timestamp(
        &self,
        timestamp: u64,
    ) -> Result<Vec<ExpirationDateSignature>> {
        let maybe_keypair_guard = self.ecash_keypair.get().await;
        let Some(keypair_guard) = maybe_keypair_guard.as_ref() else {
            return Err(CoconutError::KeyPairNotDerivedYet);
        };
        let Some(signing_key) = keypair_guard.as_ref() else {
            return Err(CoconutError::KeyPairNotDerivedYet);
        };

        Ok(sign_expiration_date(
            signing_key.keys.secret_key(),
            timestamp,
        )?)
    }

    pub async fn add_spent_credentials(&self, serial_number_bs58: &String) {
        let mut filter = self.spent_credentials.write().await;
        filter.set(serial_number_bs58);
    }

    pub async fn export_spent_credentials(&self) -> Vec<u8> {
        let filter = self.spent_credentials.read().await;
        filter.bitmap()
    }

    //SW NOTE: will be used eventually
    #[allow(dead_code)]
    pub async fn clear_spent_credentials(&self) {
        let mut filter = self.spent_credentials.write().await;
        filter.clear()
    }
}
