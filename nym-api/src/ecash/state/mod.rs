// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::client::Client as LocalClient;
use crate::ecash::comm::APICommunicationChannel;
use crate::ecash::deposit::validate_deposit;
use crate::ecash::error::{EcashError, RedemptionError, Result};
use crate::ecash::helpers::{IssuedCoinIndicesSignatures, IssuedExpirationDateSignatures};
use crate::ecash::keys::KeyPair;
use crate::ecash::state::auxiliary::AuxiliaryEcashState;
use crate::ecash::state::cleaner::EcashBackgroundStateCleaner;
use crate::ecash::state::global::GlobalEcachState;
use crate::ecash::state::helpers::{ensure_sane_expiration_date, query_all_threshold_apis};
use crate::ecash::state::local::{DailyMerkleTree, LocalEcashState};
use crate::ecash::storage::models::{SerialNumberWrapper, TicketProvider};
use crate::ecash::storage::EcashStorageExt;
use crate::support::config::Config;
use crate::support::storage::NymApiStorage;
use cosmwasm_std::{from_json, CosmosMsg, WasmMsg};
use cw3::Status;
use nym_api_requests::ecash::models::{
    BatchRedeemTicketsBody, IssuedTicketbooksChallengeCommitmentRequest,
    IssuedTicketbooksChallengeCommitmentResponseBody, IssuedTicketbooksCountResponse,
    IssuedTicketbooksDataRequest, IssuedTicketbooksDataResponseBody,
    IssuedTicketbooksForCountResponse, IssuedTicketbooksForResponseBody,
    IssuedTicketbooksOnCountResponse,
};
use nym_api_requests::ecash::BlindSignRequestBody;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::scheme::coin_indices_signatures::{
    aggregate_annotated_indices_signatures, sign_coin_indices, CoinIndexSignatureShare,
};
use nym_compact_ecash::scheme::expiration_date_signatures::{
    aggregate_annotated_expiration_signatures, ExpirationDateSignatureShare,
};
use nym_compact_ecash::{
    scheme::expiration_date_signatures::sign_expiration_date, BlindedSignature, Bytable,
    SecretKeyAuth, VerificationKeyAuth,
};
use nym_credentials::ecash::utils::EcashTime;
use nym_credentials::{aggregate_verification_keys, CredentialSpendingData};
use nym_crypto::asymmetric::ed25519;
use nym_ecash_contract_common::deposit::{Deposit, DepositId};
use nym_ecash_contract_common::msg::ExecuteMsg;
use nym_ecash_contract_common::redeem_credential::BATCH_REDEMPTION_PROPOSAL_TITLE;
use nym_ecash_time::{ecash_default_expiration_date, ecash_today_date};
use nym_task::TaskClient;
use nym_ticketbooks_merkle::{IssuedTicketbook, IssuedTicketbooksFullMerkleProof, MerkleLeaf};
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::EcashApiClient;
use rand::{thread_rng, RngCore};
use std::collections::HashMap;
use std::ops::Deref;
use time::{Date, OffsetDateTime};
use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

pub(crate) mod auxiliary;
mod cleaner;
pub(crate) mod global;
mod helpers;
pub(crate) mod local;

pub struct EcashStateConfig {
    pub(crate) issued_ticketbooks_retention_period_days: u32,
    pub(crate) maximum_data_response_size: usize,
}

impl EcashStateConfig {
    pub(crate) fn ticketbook_retention_cutoff(&self) -> Date {
        ecash_today_date()
            - time::Duration::days(self.issued_ticketbooks_retention_period_days as i64)
    }
}

impl EcashStateConfig {
    pub(crate) fn new(global_config: &Config) -> Self {
        EcashStateConfig {
            issued_ticketbooks_retention_period_days: global_config
                .ecash_signer
                .debug
                .issued_ticketbooks_retention_period_days,
            maximum_data_response_size: global_config
                .ecash_signer
                .debug
                .maximum_size_of_data_request,
        }
    }
}

#[derive(Default)]
pub(crate) enum BackgroundCleanerState {
    WaitingStartup(EcashBackgroundStateCleaner),
    Running {
        _handle: JoinHandle<()>,
    },

    // an ephemeral state so that we could swap between the other two
    #[default]
    Invalid,
}

pub struct EcashState {
    // additional global config parameters
    pub(crate) config: EcashStateConfig,

    pub(crate) background_cleaner_state: BackgroundCleanerState,

    // state global to the system, like aggregated keys, addresses, etc.
    pub(crate) global: GlobalEcachState,

    // state local to the api instance, like partial signatures, keys, etc.
    pub(crate) local: LocalEcashState,

    // auxiliary data used for resolving requests like clients, storage, etc.
    pub(crate) aux: AuxiliaryEcashState,
}

impl EcashState {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new<C, D>(
        global_config: &Config,
        contract_address: AccountId,
        client: C,
        identity_keypair: ed25519::KeyPair,
        key_pair: KeyPair,
        comm_channel: D,
        storage: NymApiStorage,
        task_client: TaskClient,
    ) -> Self
    where
        C: LocalClient + Send + Sync + 'static,
        D: APICommunicationChannel + Send + Sync + 'static,
    {
        Self {
            config: EcashStateConfig::new(global_config),
            background_cleaner_state: BackgroundCleanerState::WaitingStartup(
                EcashBackgroundStateCleaner::new(global_config, storage.clone(), task_client),
            ),
            global: GlobalEcachState::new(contract_address),
            local: LocalEcashState::new(
                key_pair,
                identity_keypair,
                !global_config.ecash_signer.enabled,
            ),
            aux: AuxiliaryEcashState::new(client, comm_channel, storage),
        }
    }

    // whilst we normally don't want to panic, this one would only occur at startup,
    // if some logical invariants got broken (which have to be fixed in code anyway)
    #[allow(clippy::panic)]
    pub(crate) fn spawn_background_cleaner(&mut self) {
        match std::mem::take(&mut self.background_cleaner_state) {
            BackgroundCleanerState::WaitingStartup(cleaner) => {
                self.background_cleaner_state = BackgroundCleanerState::Running {
                    _handle: cleaner.start(),
                }
            }
            _ => panic!("attempted to spawn background cleaner more than once"),
        }
    }

    /// Ensures that this nym-api is one of ecash signers for the current epoch
    pub(crate) async fn ensure_signer(&self) -> Result<()> {
        if self.local.explicitly_disabled {
            return Err(EcashError::NotASigner);
        }

        let epoch_id = self.aux.current_epoch().await?;

        let is_epoch_signer = self
            .local
            .active_signer
            .get_or_init(epoch_id, || async {
                let Ok(address) = self.aux.client.address().await else {
                    return Ok(false);
                };
                let ecash_signers = self.aux.comm_channel.ecash_clients(epoch_id).await?;

                // check if any ecash signers for this epoch has the same cosmos address as this api
                Ok(ecash_signers.iter().any(|c| c.cosmos_address == address))
            })
            .await?;

        if !is_epoch_signer.deref() {
            return Err(EcashError::NotASigner);
        }

        Ok(())
    }

    pub(crate) async fn ecash_signing_key(&self) -> Result<RwLockReadGuard<SecretKeyAuth>> {
        self.local.ecash_keypair.signing_key().await
    }

    #[allow(dead_code)]
    pub(crate) async fn current_master_verification_key(
        &self,
    ) -> Result<RwLockReadGuard<VerificationKeyAuth>> {
        self.master_verification_key(None).await
    }

    pub(crate) async fn master_verification_key(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<VerificationKeyAuth>> {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.aux.current_epoch().await?,
        };

        self.global
            .master_verification_key
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(stored) = self
                    .aux
                    .storage
                    .get_master_verification_key(epoch_id)
                    .await?
                {
                    return Ok(stored);
                }

                // 2. perform actual aggregation
                let all_apis = self.aux.comm_channel.ecash_clients(epoch_id).await?;
                let threshold = self.aux.comm_channel.ecash_threshold(epoch_id).await?;

                if all_apis.len() < threshold as usize {
                    return Err(EcashError::InsufficientNumberOfShares {
                        threshold,
                        shares: all_apis.len(),
                    });
                }

                let master_key = aggregate_verification_keys(&all_apis)?;

                // 3. save the key in the storage for when we reboot
                self.aux
                    .storage
                    .insert_master_verification_key(epoch_id, &master_key)
                    .await?;

                Ok(master_key)
            })
            .await
    }

    pub(crate) async fn master_coin_index_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<IssuedCoinIndicesSignatures>> {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.aux.current_epoch().await?,
        };

        self.global
            .coin_index_signatures
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(master_sigs) = self
                    .aux
                    .storage
                    .get_master_coin_index_signatures(epoch_id)
                    .await?
                {
                    return Ok(IssuedCoinIndicesSignatures {
                        epoch_id,
                        signatures: master_sigs,
                    });
                }

                info!(
                    "attempting to establish master coin index signatures for epoch {epoch_id}..."
                );

                // 2. go around APIs and attempt to aggregate the data
                let master_vk = self.master_verification_key(Some(epoch_id)).await?;
                let all_apis = self.aux.comm_channel.ecash_clients(epoch_id).await?;
                let threshold = self.aux.comm_channel.ecash_threshold(epoch_id).await?;

                // let mut shares = Mutex::new(Vec::with_capacity(all_apis.len()));
                let cosmos_address = self.aux.client.address().await.ok();

                let get_partial_signatures = |api: EcashApiClient| async {
                    // move the api into the closure
                    let api = api;
                    let node_index = api.node_id;
                    let partial_vk = api.verification_key;

                    // check if we're attempting to query ourselves, in that case just get local signature
                    // rather than making the http query
                    let partial = if Some(api.cosmos_address) == cosmos_address {
                        self.partial_coin_index_signatures(Some(epoch_id))
                            .await?
                            .signatures
                            .clone()
                    } else {
                        api.api_client
                            .partial_coin_indices_signatures(Some(epoch_id))
                            .await?
                            .signatures
                    };
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

                // 3. save the signatures in the storage for when we reboot
                self.aux
                    .storage
                    .insert_master_coin_index_signatures(epoch_id, &aggregated)
                    .await?;

                Ok(IssuedCoinIndicesSignatures {
                    epoch_id,
                    signatures: aggregated,
                })
            })
            .await
    }

    pub(crate) async fn partial_coin_index_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<IssuedCoinIndicesSignatures>> {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.aux.current_epoch().await?,
        };

        self.local
            .partial_coin_index_signatures
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(partial_sigs) = self
                    .aux
                    .storage
                    .get_partial_coin_index_signatures(epoch_id)
                    .await?
                {
                    return Ok(IssuedCoinIndicesSignatures {
                        epoch_id,
                        signatures: partial_sigs,
                    });
                }


                // 2. perform actual issuance
                let signing_keys = self.local.ecash_keypair.keys().await?;
                if signing_keys.issued_for_epoch != epoch_id {
                    // TODO: this should get handled at some point,
                    // because if it was a past epoch we **do** have those keys.
                    // they're just archived

                    error!("received partial coin index signature request for an invalid epoch ({epoch_id}). our key was derived for epoch {}", signing_keys.issued_for_epoch);
                    return Err(EcashError::InvalidSigningKeyEpoch {
                        requested: epoch_id,
                        available: signing_keys.issued_for_epoch,
                    })
                }
                let master_vk = self.master_verification_key(Some(epoch_id)).await?;
                let signatures = sign_coin_indices(
                    nym_compact_ecash::ecash_parameters(),
                    &master_vk,
                    signing_keys.keys.secret_key(),
                )?;

                // 3. save the signatures in the storage for when we reboot
                self.aux.storage.insert_partial_coin_index_signatures(epoch_id, &signatures).await?;

                Ok(IssuedCoinIndicesSignatures {
                    epoch_id,
                    signatures,
                })
            })
            .await
    }

    pub(crate) async fn master_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<IssuedExpirationDateSignatures>> {
        self.global
            .expiration_date_signatures
            .get_or_init(expiration_date, || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(master_sigs) = self
                    .aux
                    .storage
                    .get_master_expiration_date_signatures(expiration_date)
                    .await?
                {
                    return Ok(master_sigs);
                }

                // 3. go around APIs and attempt to aggregate the data
                let epoch_id = self.aux.comm_channel.current_epoch().await?;
                let master_vk = self.master_verification_key(Some(epoch_id)).await?;
                let all_apis = self.aux.comm_channel.ecash_clients(epoch_id).await?;
                let threshold = self.aux.comm_channel.ecash_threshold(epoch_id).await?;

                let cosmos_address = self.aux.client.address().await.ok();

                let get_partial_signatures = |api: EcashApiClient| async {
                    // move the api into the closure
                    let api = api;
                    let node_index = api.node_id;
                    let partial_vk = api.verification_key;

                    // check if we're attempting to query ourselves, in that case just get local signature
                    // rather than making the http query
                    let partial = if Some(api.cosmos_address) == cosmos_address {
                        self.partial_expiration_date_signatures(expiration_date)
                            .await?
                            .signatures
                            .clone()
                    } else {
                        api.api_client
                            .partial_expiration_date_signatures(Some(expiration_date))
                            .await?
                            .signatures
                    };
                    Ok(ExpirationDateSignatureShare {
                        index: node_index,
                        key: partial_vk,
                        signatures: partial,
                    })
                };

                let shares =
                    query_all_threshold_apis(all_apis, threshold, get_partial_signatures).await?;

                let aggregated = aggregate_annotated_expiration_signatures(
                    &master_vk,
                    expiration_date.ecash_unix_timestamp(),
                    &shares,
                )?;

                let issued = IssuedExpirationDateSignatures {
                    epoch_id,
                    signatures: aggregated,
                };

                // 4. save the signatures in the storage for when we reboot
                self.aux
                    .storage
                    .insert_master_expiration_date_signatures(expiration_date, &issued)
                    .await?;

                Ok(issued)
            })
            .await
    }

    pub(crate) async fn partial_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<IssuedExpirationDateSignatures>> {
        self.local
            .partial_expiration_date_signatures
            .get_or_init(expiration_date, || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(partial_sigs) = self
                    .aux
                    .storage
                    .get_partial_expiration_date_signatures(expiration_date)
                    .await?
                {
                    return Ok(partial_sigs);
                }

                // 3. perform actual issuance
                let signing_keys = self.local.ecash_keypair.keys().await?;

                let signatures = sign_expiration_date(
                    signing_keys.keys.secret_key(),
                    expiration_date.ecash_unix_timestamp(),
                )?;

                let issued = IssuedExpirationDateSignatures {
                    epoch_id: signing_keys.issued_for_epoch,
                    signatures,
                };

                // 4. save the signatures in the storage for when we reboot
                self.aux
                    .storage
                    .insert_partial_expiration_date_signatures(expiration_date, &issued)
                    .await?;

                Ok(issued)
            })
            .await
    }

    pub(crate) async fn ensure_dkg_not_in_progress(&self) -> Result<()> {
        if self.aux.comm_channel.dkg_in_progress().await? {
            return Err(EcashError::DkgInProgress);
        }
        Ok(())
    }

    /// Check if this nym-api has already issued a credential for the provided deposit id.
    /// If so, return it.
    pub async fn already_issued(&self, deposit_id: DepositId) -> Result<Option<BlindedSignature>> {
        Ok(self
            .aux
            .storage
            .get_issued_partial_signature(deposit_id)
            .await?)
    }

    pub async fn get_deposit(&self, deposit_id: DepositId) -> Result<Deposit> {
        self.aux
            .client
            .get_deposit(deposit_id)
            .await?
            .deposit
            .ok_or(EcashError::NonExistentDeposit { deposit_id })
    }

    pub async fn validate_request(
        &self,
        request: &BlindSignRequestBody,
        deposit: Deposit,
    ) -> Result<()> {
        validate_deposit(request, deposit).await
    }

    pub(crate) async fn validate_redemption_proposal(
        &self,
        request: &BatchRedeemTicketsBody,
    ) -> std::result::Result<(), RedemptionError> {
        let proposal_id = request.proposal_id;

        // retrieve the proposal itself
        let mut proposal = self
            .aux
            .client
            .get_proposal(proposal_id)
            .await
            .map_err(|_| RedemptionError::ProposalRetrievalFailure { proposal_id })?;

        if proposal.title != BATCH_REDEMPTION_PROPOSAL_TITLE {
            return Err(RedemptionError::InvalidProposalTitle {
                proposal_id,
                received: proposal.title,
            });
        }

        // make sure you can still vote on it
        match proposal.status {
            Status::Pending => return Err(RedemptionError::StillPending { proposal_id }),
            Status::Open => {}
            Status::Rejected => return Err(RedemptionError::AlreadyRejected { proposal_id }),

            // TODO: need to double check with the multisig whether it wouldn't always be thrown on threshold
            // i.e. whether after the 2+/3 vote, the remaining 1-/3 would return this error
            Status::Passed => return Err(RedemptionError::AlreadyPassed { proposal_id }),
            Status::Executed => return Err(RedemptionError::AlreadyExecuted { proposal_id }),
        }

        let encoded_digest = bs58::encode(&request.digest).into_string();

        // check if the description matches the expected digest
        if encoded_digest != proposal.description {
            return Err(RedemptionError::InvalidProposalDescription {
                proposal_id,
                received: proposal.description,
                expected: encoded_digest,
            });
        }

        // check if it was actually created by the ecash contract
        if proposal.proposer.as_str() != self.global.contract_address.as_ref() {
            return Err(RedemptionError::InvalidProposer {
                proposal_id,
                received: proposal.proposer.into_string(),
                expected: self.global.contract_address.clone(),
            });
        }

        // check if contains exactly the content we expect,
        // i.e. single `RedeemTickets` message with no funds, etc.
        if proposal.msgs.len() != 1 {
            return Err(RedemptionError::TooManyMessages { proposal_id });
        }

        // SAFETY: we just checked we have exactly one message
        #[allow(clippy::unwrap_used)]
        let msg = proposal.msgs.pop().unwrap();
        let CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds,
        }) = msg
        else {
            return Err(RedemptionError::InvalidMessage { proposal_id });
        };

        if !funds.is_empty() {
            return Err(RedemptionError::InvalidMessage { proposal_id });
        }

        if contract_addr != self.global.contract_address.as_ref() {
            return Err(RedemptionError::InvalidContract { proposal_id });
        }

        let Ok(ExecuteMsg::RedeemTickets { n, gw }) = from_json(&msg) else {
            return Err(RedemptionError::InvalidMessage { proposal_id });
        };

        if gw != request.gateway_cosmos_addr.as_ref() {
            return Err(RedemptionError::InvalidRedemptionTarget {
                proposal_id,
                proposed: gw,
                received: request.gateway_cosmos_addr.to_string(),
            });
        }

        if n as usize != request.included_serial_numbers.len() {
            return Err(RedemptionError::InvalidRedemptionTicketCount {
                proposal_id,
                proposed: n,
                received: request.included_serial_numbers.len() as u16,
            });
        }

        Ok(())
    }

    pub(crate) async fn accept_proposal(&self, proposal_id: u64) -> Result<()> {
        //SW NOTE: What to do if this fails
        if let Err(err) = self.aux.client.vote_proposal(proposal_id, true, None).await {
            debug!("failed to vote on proposal {proposal_id}: {err}");
        }

        Ok(())
    }

    // pub(crate) async fn blacklist(&self, public_key: String) {
    //     let client = self.aux.client.clone();
    //     tokio::spawn(async move {
    //         //SW TODO error handling with one log at the end
    //         let response = client.propose_for_blacklist(public_key.clone()).await?;
    //         let proposal_id = find_proposal_id(&response.logs)?;
    //
    //         let proposal = client.get_proposal(proposal_id).await?;
    //         if proposal.status == Status::Open {
    //             if public_key != proposal.description {
    //                 return Err(EcashError::IncorrectProposal {
    //                     reason: String::from("incorrect publickey in description"),
    //                 });
    //             }
    //             let ret = client.vote_proposal(proposal_id, true, None).await;
    //
    //             accepted_vote_err(ret)?;
    //
    //             if let Ok(proposal) = client.get_proposal(proposal_id).await {
    //                 if proposal.status == Status::Passed {
    //                     client.execute_proposal(proposal_id).await?
    //                 }
    //             }
    //         }
    //         Ok(())
    //     });
    // }

    pub(crate) async fn persist_issued(
        &self,
        current_epoch: EpochId,
        issued: &IssuedTicketbook,
        merkle_leaf: MerkleLeaf,
    ) -> Result<()> {
        // note: we have a UNIQUE constraint on the deposit_id column of the credential
        // and so if the api is processing request for the same deposit at the same time,
        // only one of them will be successfully inserted to the database
        self.aux
            .storage
            .store_issued_ticketbook(
                issued.deposit_id,
                current_epoch as u32,
                &issued.blinded_partial_credential,
                &issued.joined_encoded_private_attributes_commitments,
                issued.expiration_date,
                issued.ticketbook_type,
                merkle_leaf,
            )
            .await?;
        Ok(())
    }

    async fn get_updated_merkle_read(
        &self,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<DailyMerkleTree>> {
        let write_guard = self.get_updated_full_write(expiration_date).await?;

        // SAFETY: the entry was either not empty or we just inserted data in there, whilst never dropping the lock
        // thus it MUST exist
        #[allow(clippy::unwrap_used)]
        Ok(RwLockWriteGuard::downgrade_map(write_guard, |map| {
            map.get(&expiration_date).unwrap()
        }))
    }

    async fn get_updated_full_write(
        &self,
        expiration_date: Date,
    ) -> Result<RwLockWriteGuard<HashMap<Date, DailyMerkleTree>>> {
        let mut write_guard = self.local.issued_merkle_trees.write().await;

        // double check if it's still empty in case another task has already grabbed the write lock and performed the update
        let still_empty = write_guard.get(&expiration_date).is_none();
        if still_empty {
            // the order actually does not matter since we're building the tree back from scratch
            let issued_hashes = self.aux.storage.get_issued_hashes(expiration_date).await?;
            write_guard.insert(expiration_date, DailyMerkleTree::new(issued_hashes));
        }
        Ok(write_guard)
    }

    pub async fn store_issued_ticketbook(
        &self,
        request_body: BlindSignRequestBody,
        blinded_signature: &BlindedSignature,
    ) -> Result<()> {
        let current_epoch = self.aux.current_epoch().await?;
        let expiration = request_body.expiration_date;
        let deposit_id = request_body.deposit_id;

        let joined_encoded_private_attributes_commitments = request_body.encode_join_commitments();
        let issued = IssuedTicketbook {
            deposit_id: request_body.deposit_id,
            epoch_id: current_epoch,
            blinded_partial_credential: blinded_signature.to_byte_vec(),
            joined_encoded_private_attributes_commitments,
            expiration_date: request_body.expiration_date,
            ticketbook_type: request_body.ticketbook_type,
        };

        let mut map = self.get_updated_full_write(expiration).await?;
        // SAFETY: get_updated_full_write inserted relevant entry to the map, and we never dropped the lock
        #[allow(clippy::unwrap_used)]
        let merkle_entry = map.get_mut(&expiration).unwrap();

        // insert the ticketbook into the merkle tree
        let inserted_leaf = merkle_entry.insert(&issued);

        // note: there's a primary key constraint on the deposit_id
        // and so if the api is processing request for the same deposit at the same time,
        // only one of them will be successfully inserted to the database
        if let Err(err) = self
            .persist_issued(current_epoch, &issued, inserted_leaf)
            .await
        {
            // if we failed to insert it into the db, rollback the tree. there was most likely clash on the deposit
            warn!("failed to persist ticketbook corresponding to deposit {deposit_id}: {err}");
            merkle_entry.rollback(deposit_id);
            return Err(err);
        }

        // if we managed to insert it into db, check if we might want to purge the tree history,
        // since we will no longer have to roll it back
        merkle_entry.maybe_rebuild();

        // toss a coin to check if we should clean memory of old merkle trees
        if thread_rng().next_u32() % 10000 == 0 {
            let mut values_to_clean = Vec::new();
            let cutoff = self.config.ticketbook_retention_cutoff();
            info!("attempting to remove old issued ticketbooks. the cutoff is set to {cutoff}");

            for date in map.keys() {
                if date < &cutoff {
                    values_to_clean.push(*date)
                }
            }

            for date in values_to_clean {
                // remove the in-memory merkle tree
                map.remove(&date);
            }
        }

        Ok(())
    }

    async fn get_merkle_proof(
        &self,
        expiration_date: Date,
        deposits: &[DepositId],
    ) -> Result<IssuedTicketbooksFullMerkleProof> {
        // check if the entry for this expiration date is empty. if so, it might imply we have crashed/shutdown
        // and not have the full data in memory
        if self.local.is_merkle_empty(expiration_date).await {
            let entry = self.get_updated_merkle_read(expiration_date).await?;

            return entry.proof(deposits);
        }

        // I can imagine this could happen under very rare edge case when the function is called just as the retention period expired
        let guard = self.local.issued_merkle_trees.read().await;
        let Some(entry) = guard.get(&expiration_date) else {
            warn!("it seems our merkle tree has just expired!");
            return Err(EcashError::ExpirationDateTooEarly);
        };
        entry.proof(deposits)
    }

    pub async fn get_issued_ticketbooks_challenge_commitment(
        &self,
        challenge: IssuedTicketbooksChallengeCommitmentRequest,
    ) -> Result<IssuedTicketbooksChallengeCommitmentResponseBody> {
        let body = &challenge.body;
        if body.expiration_date < self.config.ticketbook_retention_cutoff() {
            return Err(EcashError::ExpirationDateTooEarly);
        }

        if body.expiration_date > ecash_default_expiration_date() {
            // we wouldn't have issued any credentials for that expiration date so no point
            // in attempting to construct an ultimately empty response
            return Err(EcashError::ExpirationDateTooLate);
        }

        let merkle_proof = self
            .get_merkle_proof(body.expiration_date, &body.deposits)
            .await?;

        Ok(IssuedTicketbooksChallengeCommitmentResponseBody {
            expiration_date: body.expiration_date,
            original_request: challenge,
            max_data_response_size: self.config.maximum_data_response_size,
            merkle_proof,
        })
    }

    pub async fn get_issued_ticketbooks_data(
        &self,
        request: IssuedTicketbooksDataRequest,
    ) -> Result<IssuedTicketbooksDataResponseBody> {
        let body = &request.body;
        if body.expiration_date < self.config.ticketbook_retention_cutoff() {
            return Err(EcashError::ExpirationDateTooEarly);
        }

        if body.expiration_date > ecash_default_expiration_date() {
            // we wouldn't have issued any credentials for that expiration date so no point
            // in attempting to construct an ultimately empty response
            return Err(EcashError::ExpirationDateTooLate);
        }

        // prevent ddos attacks by allowing requesters to force us to load all the ticketbooks into memory
        if body.deposits.len() > self.config.maximum_data_response_size {
            return Err(EcashError::RequestTooBig {
                requested: body.deposits.len(),
                max: self.config.maximum_data_response_size,
            });
        }

        let partial_ticketbooks = self
            .aux
            .storage
            .get_issued_ticketbooks(&body.deposits)
            .await?
            .into_iter()
            .map(|t| (t.deposit_id, t))
            .collect();

        Ok(IssuedTicketbooksDataResponseBody {
            expiration_date: body.expiration_date,
            partial_ticketbooks,
            original_request: request,
        })
    }

    pub async fn get_issued_ticketbooks_deposits_on(
        &self,
        expiration: Date,
    ) -> Result<IssuedTicketbooksForResponseBody> {
        if expiration < self.config.ticketbook_retention_cutoff() {
            return Err(EcashError::ExpirationDateTooEarly);
        }

        // add some leeway
        if expiration > ecash_default_expiration_date() + time::Duration::days(2) {
            return Err(EcashError::ExpirationDateTooLate);
        }

        // check if the entry for this expiration date is empty. if so, it might imply we have crashed/shutdown
        // and not have the full data in memory
        if self.local.is_merkle_empty(expiration).await {
            let entry = self.get_updated_merkle_read(expiration).await?;

            return Ok(IssuedTicketbooksForResponseBody {
                expiration_date: expiration,
                deposits: entry.deposits(),
                merkle_root: entry.merkle_root(),
            });
        }

        // I can imagine this could happen under very rare edge case when the function is called just as the retention period expired
        let guard = self.local.issued_merkle_trees.read().await;
        let Some(entry) = guard.get(&expiration) else {
            warn!("it seems our merkle tree has just expired!");
            return Err(EcashError::ExpirationDateTooEarly);
        };

        Ok(IssuedTicketbooksForResponseBody {
            expiration_date: expiration,
            deposits: entry.deposits(),
            merkle_root: entry.merkle_root(),
        })
    }

    /// Returns a boolean to indicate whether the ticket has actually been inserted
    pub async fn store_verified_ticket(
        &self,
        ticket_data: &CredentialSpendingData,
        gateway_addr: &AccountId,
    ) -> Result<bool> {
        self.aux
            .storage
            .store_verified_ticket(ticket_data, gateway_addr)
            .await
            .map_err(Into::into)
    }

    pub async fn get_ticket_provider(
        &self,
        gateway_address: &str,
    ) -> Result<Option<TicketProvider>> {
        self.aux
            .storage
            .get_ticket_provider(gateway_address)
            .await
            .map_err(Into::into)
    }

    pub async fn get_redeemable_tickets(
        &self,
        provider_info: &TicketProvider,
    ) -> Result<Vec<SerialNumberWrapper>> {
        let since = provider_info
            .last_batch_verification
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);

        self.aux
            .storage
            .get_verified_tickets_since(provider_info.id, since)
            .await
            .map_err(Into::into)
    }

    pub async fn update_last_batch_verification(&self, provider: &TicketProvider) -> Result<()> {
        Ok(self
            .aux
            .storage
            .update_last_batch_verification(provider.id, OffsetDateTime::now_utc())
            .await?)
    }

    pub async fn get_ticket_data_by_serial_number(
        &self,
        serial_number: &[u8],
    ) -> Result<Option<CredentialSpendingData>> {
        self.aux
            .storage
            .get_credential_data(serial_number)
            .await
            .map_err(Into::into)
    }

    pub async fn get_issued_ticketbooks_count(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<IssuedTicketbooksCountResponse> {
        // convert to db offset
        // we're paging from page 0 like civilised people,
        // so we have to skip (page * per_page) results
        let offset = page * per_page;
        let limit = per_page;

        let issued = self
            .aux
            .storage
            .get_issued_ticketbooks_count(limit, offset)
            .await?;

        Ok(IssuedTicketbooksCountResponse {
            issued: issued.into_iter().map(Into::into).collect(),
        })
    }

    pub async fn get_issued_ticketbooks_on_count(
        &self,
        issuance_date: Date,
    ) -> Result<IssuedTicketbooksOnCountResponse> {
        let issued = self
            .aux
            .storage
            .get_issued_ticketbooks_on_count(issuance_date)
            .await?;

        Ok(IssuedTicketbooksOnCountResponse {
            issuance_date,
            total: issued.iter().map(|count| count.count as usize).sum(),
            issued: issued.into_iter().map(Into::into).collect(),
        })
    }

    pub async fn get_issued_ticketbooks_for_count(
        &self,
        expiration_date: Date,
    ) -> Result<IssuedTicketbooksForCountResponse> {
        let issued = self
            .aux
            .storage
            .get_issued_ticketbooks_for_count(expiration_date)
            .await?;

        Ok(IssuedTicketbooksForCountResponse {
            expiration_date,
            total: issued.iter().map(|count| count.count as usize).sum(),
            issued: issued.into_iter().map(Into::into).collect(),
        })
    }
}
