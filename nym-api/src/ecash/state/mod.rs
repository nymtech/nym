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
use nym_task::ShutdownManager;
use nym_ticketbooks_merkle::{IssuedTicketbook, IssuedTicketbooksFullMerkleProof, MerkleLeaf};
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::EcashApiClient;
use rand::{thread_rng, RngCore};
use std::collections::HashMap;
use std::sync::Arc;
use time::{Date, OffsetDateTime};
use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

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
        identity_keypair: Arc<ed25519::KeyPair>,
        key_pair: KeyPair,
        comm_channel: D,
        storage: NymApiStorage,
        shutdown_manager: &ShutdownManager,
    ) -> Self
    where
        C: LocalClient + Send + Sync + 'static,
        D: APICommunicationChannel + Send + Sync + 'static,
    {
        Self {
            config: EcashStateConfig::new(global_config),
            background_cleaner_state: BackgroundCleanerState::WaitingStartup(
                EcashBackgroundStateCleaner::new(
                    global_config,
                    storage.clone(),
                    shutdown_manager.clone_shutdown_token(),
                ),
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

    pub(crate) async fn current_dkg_epoch(&self) -> Result<EpochId> {
        self.aux.current_epoch().await
    }

    async fn check_dkg_signer(&self, epoch_id: EpochId) -> Result<bool> {
        let Ok(address) = self.aux.client.address().await else {
            return Ok(false);
        };
        let ecash_signers = self.aux.comm_channel.ecash_clients(epoch_id).await?;

        // check if any ecash signers for this epoch has the same cosmos address as this api
        Ok(ecash_signers.iter().any(|c| c.cosmos_address == address))
    }

    pub(crate) async fn is_dkg_signer(&self, epoch_id: EpochId) -> Result<bool> {
        // our own membership is only settled once the ceremony has concluded. this cache
        // never expires, so answering "not a signer" mid-ceremony and remembering it
        // would have us refuse to sign for the rest of the epoch.
        if !self.aux.comm_channel.ceremony_concluded(epoch_id).await? {
            return self.check_dkg_signer(epoch_id).await;
        }

        let is_epoch_signer = self
            .local
            .active_signer
            .get_or_init(epoch_id, || async { self.check_dkg_signer(epoch_id).await })
            .await?;
        Ok(*is_epoch_signer)
    }

    /// Ensures that this nym-api is one of ecash signers for the current epoch
    pub(crate) async fn ensure_signer(&self) -> Result<()> {
        let epoch_id = self.current_dkg_epoch().await?;
        self.ensure_signer_for_epoch(epoch_id).await
    }

    /// Ensures that this nym-api was one of the ecash signers for the given epoch.
    ///
    /// Credentials outlive the epoch that issued them, so the material they need is asked of
    /// *that* epoch's signers - which is not necessarily who signs today. An api that has since
    /// dropped out of the set still holds the keys, and refusing on the strength of the current
    /// epoch alone can leave a past epoch permanently short of the threshold it needs.
    pub(crate) async fn ensure_signer_for_epoch(&self, epoch_id: EpochId) -> Result<()> {
        if self.local.explicitly_disabled {
            return Err(EcashError::NotASigner);
        }

        if !self.is_dkg_signer(epoch_id).await? {
            return Err(EcashError::NotASigner);
        }

        Ok(())
    }

    pub(crate) async fn ecash_signing_key(&self) -> Result<RwLockReadGuard<'_, SecretKeyAuth>> {
        self.local.ecash_keypair.signing_key().await
    }

    #[allow(dead_code)]
    pub(crate) async fn current_master_verification_key(
        &self,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>> {
        self.master_verification_key(None).await
    }

    pub(crate) async fn master_verification_key(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>> {
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
    ) -> Result<RwLockReadGuard<'_, IssuedCoinIndicesSignatures>> {
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
    ) -> Result<RwLockReadGuard<'_, IssuedCoinIndicesSignatures>> {
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
                //
                // a past epoch is answered from the key it was signed with, which we archived
                // rather than destroyed when it rotated. what makes that safe is the epoch's
                // ceremony being over, so check exactly that rather than inheriting the
                // "may we issue right now" flag, which a later ceremony clears
                self.ensure_ceremony_concluded(epoch_id).await?;
                let signing_keys = self.local.ecash_keypair.keys_for_epoch(epoch_id).await?;
                let master_vk = self.master_verification_key(Some(epoch_id)).await?;
                let signatures = sign_coin_indices(
                    nym_compact_ecash::ecash_parameters(),
                    &master_vk,
                    signing_keys.keys.secret_key(),
                )?;

                // 3. save the signatures in the storage for when we reboot
                self.aux
                    .storage
                    .insert_partial_coin_index_signatures(epoch_id, &signatures)
                    .await?;

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
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, IssuedExpirationDateSignatures>> {
        self.global
            .expiration_date_signatures
            .get_or_init((epoch_id, expiration_date), || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(master_sigs) = self
                    .aux
                    .storage
                    .get_master_expiration_date_signatures(expiration_date, epoch_id)
                    .await?
                {
                    return Ok(master_sigs);
                }

                // 3. go around APIs and attempt to aggregate the data
                //
                // everything below has to stay on the epoch that was *asked for*: credentials
                // outlive the epoch that issued them, so answering with the current epoch's
                // material produces signatures the caller cannot verify - and the answer would
                // then be cached and persisted under the epoch it does not belong to
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
                        self.partial_expiration_date_signatures(expiration_date, epoch_id)
                            .await?
                            .signatures
                            .clone()
                    } else {
                        api.api_client
                            .partial_expiration_date_signatures(
                                Some(expiration_date),
                                Some(epoch_id),
                            )
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
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, IssuedExpirationDateSignatures>> {
        self.local
            .partial_expiration_date_signatures
            .get_or_init((epoch_id, expiration_date), || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(partial_sigs) = self
                    .aux
                    .storage
                    .get_partial_expiration_date_signatures(expiration_date, epoch_id)
                    .await?
                {
                    return Ok(partial_sigs);
                }

                // 3. perform actual issuance
                //
                // as with the coin index sibling: a settled epoch is answered from the key it
                // was signed with, and it is the ceremony being over that makes that safe
                self.ensure_ceremony_concluded(epoch_id).await?;
                let signing_keys = self.local.ecash_keypair.keys_for_epoch(epoch_id).await?;

                let signatures = sign_expiration_date(
                    signing_keys.keys.secret_key(),
                    expiration_date.ecash_unix_timestamp(),
                )?;

                let issued = IssuedExpirationDateSignatures {
                    epoch_id,
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

    /// Ensures the DKG ceremony that established `epoch_id`'s keys has finished, so everything
    /// derived from them is settled.
    ///
    /// Only the epoch whose ceremony is running right now has nothing to give. Everything an
    /// earlier one was ever asked for is fixed for good, and its credentials stay spendable for
    /// days after it stops being used for issuance - so refusing those requests for the duration
    /// of a ceremony takes credentials out of service for a reason that does not apply to them.
    pub(crate) async fn ensure_ceremony_concluded(&self, epoch_id: EpochId) -> Result<()> {
        if !self.aux.comm_channel.ceremony_concluded(epoch_id).await? {
            return Err(EcashError::CeremonyNotConcluded { epoch_id });
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
    ) -> Result<RwLockReadGuard<'_, DailyMerkleTree>> {
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
    ) -> Result<RwLockWriteGuard<'_, HashMap<Date, DailyMerkleTree>>> {
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
        if thread_rng().next_u32().is_multiple_of(10000) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecash::dkg::controller::keys::{
        archive_ecash_keypair, load_archived_ecash_keypairs, persist_ecash_keypair,
    };
    use crate::ecash::keys::KeyPairWithEpoch;
    use crate::ecash::tests::contract_chain::SharedContractChain;
    use crate::ecash::tests::contract_harness::{
        cheap, contract_backed_ecash_state, initiate_dkg, trigger_reset,
    };

    /// B10, at the layer that actually refuses service: `ensure_signer` is the first
    /// thing every ticket verification does, and gateways poll continuously, so a query
    /// landing mid-ceremony is a certainty rather than a risk.
    ///
    /// Currently RED. Note `active_signer` is a *separate* cache from the communication
    /// channel's `epoch_clients`, so fixing that one alone would not fix this.
    #[tokio::test]
    async fn a_signer_still_recognises_itself_after_a_ceremony() -> anyhow::Result<()> {
        let chain = SharedContractChain::new(3);
        initiate_dkg(&chain);
        let epoch_id = chain.epoch().epoch_id;

        // this api is one of the group members, so it will be a signer for this epoch
        let me = chain.group_member_addresses()[0].clone();
        let state = contract_backed_ecash_state(&chain, me).await;

        // something asks whether we are a signer while the ceremony is still running
        cheap::register_dealers(&chain, false);
        let _ = state.is_dkg_signer(epoch_id).await;

        cheap::advance(&chain);
        cheap::submit_dealings(&chain, false);
        let _ = state.is_dkg_signer(epoch_id).await;

        cheap::advance(&chain);
        cheap::submit_vk_shares(&chain, false);
        let _ = state.is_dkg_signer(epoch_id).await;

        cheap::advance(&chain);
        cheap::advance(&chain);
        cheap::verify_vk_shares(&chain, false);
        cheap::advance(&chain);
        cheap::install_real_verification_keys(&chain);

        // the ceremony concluded and we are one of its signers, so we must serve again
        // without being restarted
        assert!(state.is_dkg_signer(epoch_id).await?);
        state.ensure_signer().await?;

        Ok(())
    }

    /// The layer above: aggregating the epoch's master verification key depends on
    /// signer discovery, so a poisoned signer set leaves it permanently unavailable.
    ///
    /// It is guarded by a threshold check and `get_or_init` never caches errors, so it
    /// is *designed* to recover on the next request. Currently RED because the layer
    /// beneath it cannot.
    #[tokio::test]
    async fn the_master_verification_key_becomes_available_after_a_ceremony() -> anyhow::Result<()>
    {
        let chain = SharedContractChain::new(3);
        initiate_dkg(&chain);
        let epoch_id = chain.epoch().epoch_id;

        let me = chain.group_member_addresses()[0].clone();
        let state = contract_backed_ecash_state(&chain, me).await;

        // asked for too early, this must fail - there is nothing to aggregate yet
        cheap::register_dealers(&chain, false);
        assert!(state.master_verification_key(Some(epoch_id)).await.is_err());

        cheap::advance(&chain);
        cheap::submit_dealings(&chain, false);
        cheap::advance(&chain);
        cheap::submit_vk_shares(&chain, false);
        cheap::advance(&chain);
        cheap::advance(&chain);
        cheap::verify_vk_shares(&chain, false);
        cheap::advance(&chain);
        let expected = cheap::install_real_verification_keys(&chain);

        // ... but once the ceremony concludes it must resolve, and to the right key
        let recovered = state.master_verification_key(Some(epoch_id)).await?;
        assert_eq!(*recovered, expected.master);

        Ok(())
    }

    /// The threshold cache sits alongside the poisoned ones but is not poisonable: the
    /// contract has no threshold until dealing exchange begins, and an absent threshold
    /// is an *error*, which `get_or_init` never caches. Pinned so that a future change
    /// returning a placeholder instead of an error does not quietly introduce the bug.
    #[tokio::test]
    async fn the_epoch_threshold_is_not_poisoned_by_an_early_query() -> anyhow::Result<()> {
        let chain = SharedContractChain::new(3);
        initiate_dkg(&chain);
        let epoch_id = chain.epoch().epoch_id;

        let me = chain.group_member_addresses()[0].clone();
        let state = contract_backed_ecash_state(&chain, me).await;

        // before dealing exchange the contract has not computed a threshold yet
        cheap::register_dealers(&chain, false);
        assert!(state
            .aux
            .comm_channel
            .ecash_threshold(epoch_id)
            .await
            .is_err());

        cheap::advance(&chain);

        // it is frozen on entry to dealing exchange, and the earlier failure did not stick
        assert_eq!(
            state.aux.comm_channel.ecash_threshold(epoch_id).await?,
            2 // ceil(2 * 3 / 3)
        );

        Ok(())
    }

    /// Credentials outlive the epoch that issued them, so an api is asked for the expiration
    /// date signatures of epochs that are no longer current, and what comes back has to verify
    /// against *that* epoch's master key. Aggregating from whatever epoch happens to be
    /// current instead produces material the caller cannot use, and the answer is then cached
    /// under the epoch that was asked for, so every later caller gets it too.
    ///
    /// A single signer keeps this local: it is the only api in the group, so aggregation reads
    /// its own partial signatures out of storage rather than going over the network.
    #[tokio::test]
    async fn expiration_date_signatures_are_aggregated_for_the_epoch_that_was_asked_for(
    ) -> anyhow::Result<()> {
        let chain = SharedContractChain::new(1);
        initiate_dkg(&chain);

        cheap::run_ceremony(&chain, false);
        let past_epoch = chain.epoch().epoch_id;
        let past_keys = cheap::install_real_verification_keys(&chain);

        // a second ceremony, so the api now holds a key unrelated to the one credentials from
        // the first epoch were issued under
        trigger_reset(&chain);
        cheap::run_ceremony(&chain, false);
        let current_epoch = chain.epoch().epoch_id;
        let current_keys = cheap::install_real_verification_keys(&chain);
        assert_ne!(past_epoch, current_epoch);
        assert_ne!(past_keys.master, current_keys.master);

        let me = chain.group_member_addresses()[0].clone();
        let state = contract_backed_ecash_state(&chain, me).await;

        // the partial signatures this api issued in each epoch, as it would have stored them
        let expiration_date = ecash_today_date();
        for (epoch_id, keys) in [(past_epoch, &past_keys), (current_epoch, &current_keys)] {
            let signatures = sign_expiration_date(
                keys.keypairs[0].secret_key(),
                expiration_date.ecash_unix_timestamp(),
            )?;
            state
                .aux
                .storage
                .insert_partial_expiration_date_signatures(
                    expiration_date,
                    &IssuedExpirationDateSignatures {
                        epoch_id,
                        signatures,
                    },
                )
                .await?;
        }

        let served = state
            .master_expiration_date_signatures(expiration_date, past_epoch)
            .await?;

        assert_eq!(
            served.epoch_id, past_epoch,
            "a request for a past epoch was answered with the current epoch's signatures"
        );

        Ok(())
    }

    /// B3: a ticketbook outlives the epoch that issued it, so after a rotation this api is
    /// still asked for that epoch's partial signatures. It archived the key rather than
    /// destroying it, so it can still produce them - and with a reset it *must*, because the
    /// master key changed and no other epoch's material will verify for those books.
    ///
    /// The archive is read back the way a restarted api reads it: off disk, by epoch.
    #[tokio::test]
    async fn partial_signatures_for_a_past_epoch_are_produced_from_the_archived_key(
    ) -> anyhow::Result<()> {
        let chain = SharedContractChain::new(1);
        initiate_dkg(&chain);

        cheap::run_ceremony(&chain, false);
        let past_epoch = chain.epoch().epoch_id;
        let past_keys = cheap::install_real_verification_keys(&chain);

        // the key this api signed the first epoch with gets archived as the next ceremony begins
        let key_dir = tempfile::tempdir()?;
        let key_path = key_dir.path().join("ecash.pem");
        persist_ecash_keypair(
            &KeyPairWithEpoch::new(past_keys.keypairs.into_iter().next().unwrap(), past_epoch),
            &key_path,
        )?;
        archive_ecash_keypair(&key_path, past_epoch)?;

        // a reset, so the master key the first epoch's credentials verify against is gone
        trigger_reset(&chain);
        cheap::run_ceremony(&chain, false);
        let current_epoch = chain.epoch().epoch_id;
        let current_keys = cheap::install_real_verification_keys(&chain);
        assert_ne!(past_keys.master, current_keys.master);

        let me = chain.group_member_addresses()[0].clone();
        let state = contract_backed_ecash_state(&chain, me).await;

        // as a restarting api would: the live key for the epoch it now signs for, plus
        // whatever it finds archived alongside it
        state
            .local
            .ecash_keypair
            .set(KeyPairWithEpoch::new(
                current_keys.keypairs.into_iter().next().unwrap(),
                current_epoch,
            ))
            .await;
        state.local.ecash_keypair.validate();
        let archived = load_archived_ecash_keypairs(&key_path);
        assert_eq!(archived.len(), 1);
        for keys in archived {
            state.local.ecash_keypair.archive(keys).await;
        }

        let expiration_date = ecash_today_date();

        // both kinds of auxiliary material must come back stamped with the epoch asked for
        let expiration_partial = state
            .partial_expiration_date_signatures(expiration_date, past_epoch)
            .await?;
        assert_eq!(expiration_partial.epoch_id, past_epoch);

        let coin_index_partial = state
            .partial_coin_index_signatures(Some(past_epoch))
            .await?;
        assert_eq!(coin_index_partial.epoch_id, past_epoch);
        drop(expiration_partial);
        drop(coin_index_partial);

        // and it has to be the *right* material: aggregation verifies each partial against
        // the epoch's master key, so signing with the current key would fail here
        let master_expiration = state
            .master_expiration_date_signatures(expiration_date, past_epoch)
            .await?;
        assert_eq!(master_expiration.epoch_id, past_epoch);
        drop(master_expiration);

        let master_coin_indices = state.master_coin_index_signatures(Some(past_epoch)).await?;
        assert_eq!(master_coin_indices.epoch_id, past_epoch);

        Ok(())
    }

    /// B2 at the layer beneath the routes: lifting the gate is only worth anything if the data
    /// can actually be produced while a ceremony runs. Everything it depends on belongs to the
    /// epoch being asked about - its signer set, its threshold, its keys - so none of it is
    /// touched by the ceremony running for the *next* epoch.
    #[tokio::test]
    async fn a_concluded_epoch_can_still_be_served_while_the_next_ceremony_runs(
    ) -> anyhow::Result<()> {
        let chain = SharedContractChain::new(1);
        initiate_dkg(&chain);

        cheap::run_ceremony(&chain, false);
        let past_epoch = chain.epoch().epoch_id;
        let past_keys = cheap::install_real_verification_keys(&chain);

        let key_dir = tempfile::tempdir()?;
        let key_path = key_dir.path().join("ecash.pem");
        persist_ecash_keypair(
            &KeyPairWithEpoch::new(past_keys.keypairs.into_iter().next().unwrap(), past_epoch),
            &key_path,
        )?;
        archive_ecash_keypair(&key_path, past_epoch)?;

        // a fresh ceremony is under way and has not produced anything yet
        trigger_reset(&chain);
        cheap::register_dealers(&chain, false);
        cheap::advance(&chain);
        let current_epoch = chain.epoch().epoch_id;
        assert_ne!(past_epoch, current_epoch);

        let me = chain.group_member_addresses()[0].clone();
        let state = contract_backed_ecash_state(&chain, me).await;
        for keys in load_archived_ecash_keypairs(&key_path) {
            state.local.ecash_keypair.archive(keys).await;
        }

        // the blanket gate would have refused everything in this situation
        assert!(state.ensure_dkg_not_in_progress().await.is_err());

        // the epoch being built has nothing to give ...
        assert!(matches!(
            state.ensure_ceremony_concluded(current_epoch).await,
            Err(EcashError::CeremonyNotConcluded { epoch_id }) if epoch_id == current_epoch
        ));

        // ... while the one that finished is settled, and every layer can still serve it
        state.ensure_ceremony_concluded(past_epoch).await?;

        let expiration_date = ecash_today_date();
        let partial = state
            .partial_expiration_date_signatures(expiration_date, past_epoch)
            .await?;
        assert_eq!(partial.epoch_id, past_epoch);
        drop(partial);

        let master = state
            .master_expiration_date_signatures(expiration_date, past_epoch)
            .await?;
        assert_eq!(master.epoch_id, past_epoch);
        drop(master);

        let coin_indices = state.master_coin_index_signatures(Some(past_epoch)).await?;
        assert_eq!(coin_indices.epoch_id, past_epoch);
        drop(coin_indices);

        let vk = state.master_verification_key(Some(past_epoch)).await?;
        assert_eq!(*vk, past_keys.master);

        Ok(())
    }

    /// The window the two tests above miss. A ceremony clears the "may we issue" flag as soon as
    /// it starts, but the keys it clears it for are not archived until dealing exchange - so for
    /// the first phase of every ceremony the previous epoch's keys sit in the live slot, unusable
    /// for issuance and not yet in the archive. Its credentials still need serving throughout.
    #[tokio::test]
    async fn a_settled_epoch_is_served_from_the_live_slot_before_its_keys_are_archived(
    ) -> anyhow::Result<()> {
        let chain = SharedContractChain::new(1);
        initiate_dkg(&chain);

        cheap::run_ceremony(&chain, false);
        let past_epoch = chain.epoch().epoch_id;
        let past_keys = cheap::install_real_verification_keys(&chain);

        let me = chain.group_member_addresses()[0].clone();
        let state = contract_backed_ecash_state(&chain, me).await;

        // the keys it signed that epoch with, in use and usable
        state
            .local
            .ecash_keypair
            .set(KeyPairWithEpoch::new(
                past_keys.keypairs.into_iter().next().unwrap(),
                past_epoch,
            ))
            .await;
        state.local.ecash_keypair.validate();

        // a ceremony begins: the flag is cleared at public key submission, but nothing has
        // been archived yet - dealing exchange is what does that
        trigger_reset(&chain);
        state.local.ecash_keypair.invalidate();
        let current_epoch = chain.epoch().epoch_id;
        assert_ne!(past_epoch, current_epoch);

        // issuance is indeed halted ...
        assert!(state.ecash_signing_key().await.is_err());

        // ... but the epoch that finished is still settled, and still has to be served
        let expiration_date = ecash_today_date();
        let partial = state
            .partial_expiration_date_signatures(expiration_date, past_epoch)
            .await?;
        assert_eq!(partial.epoch_id, past_epoch);
        drop(partial);

        let coin_indices = state
            .partial_coin_index_signatures(Some(past_epoch))
            .await?;
        assert_eq!(coin_indices.epoch_id, past_epoch);
        drop(coin_indices);

        // the epoch being built is refused, even though its keys are the ones we hold
        assert!(matches!(
            state
                .partial_expiration_date_signatures(expiration_date, current_epoch)
                .await,
            Err(EcashError::CeremonyNotConcluded { epoch_id }) if epoch_id == current_epoch
        ));

        Ok(())
    }

    /// B3, at the gate sitting in front of it: the material a past epoch's credentials need is
    /// asked of *that* epoch's signers, which is not necessarily whoever signs today. An api
    /// that has since dropped out of the set still holds the keys, and refusing it on the
    /// strength of the current epoch alone can leave a past epoch permanently short of the
    /// threshold its aggregation needs.
    #[tokio::test]
    async fn a_signer_that_left_the_set_still_answers_for_the_epoch_it_signed() -> anyhow::Result<()>
    {
        let chain = SharedContractChain::new(4);
        initiate_dkg(&chain);

        cheap::run_ceremony(&chain, false);
        let past_epoch = chain.epoch().epoch_id;
        let past_keys = cheap::install_real_verification_keys(&chain);

        // the last of the four archives the key it signed that epoch with
        let me = chain.group_member_addresses()[3].clone();
        let key_dir = tempfile::tempdir()?;
        let key_path = key_dir.path().join("ecash.pem");
        persist_ecash_keypair(
            &KeyPairWithEpoch::new(past_keys.keypairs.into_iter().nth(3).unwrap(), past_epoch),
            &key_path,
        )?;
        archive_ecash_keypair(&key_path, past_epoch)?;

        // a reset in which its share never gets verified. 3 of 4 still meets the threshold,
        // so the epoch concludes without it
        trigger_reset(&chain);
        cheap::register_dealers(&chain, false);
        cheap::advance(&chain);
        cheap::submit_dealings(&chain, false);
        cheap::advance(&chain);
        cheap::submit_vk_shares(&chain, false);
        cheap::advance(&chain);
        cheap::advance(&chain);
        cheap::verify_first_vk_shares(&chain, false, 3);
        cheap::advance(&chain);
        let current_epoch = chain.epoch().epoch_id;
        assert_ne!(past_epoch, current_epoch);
        cheap::install_real_verification_keys(&chain);

        let state = contract_backed_ecash_state(&chain, me).await;
        for keys in load_archived_ecash_keypairs(&key_path) {
            state.local.ecash_keypair.archive(keys).await;
        }

        // it is not one of today's signers ...
        assert!(matches!(
            state.ensure_signer().await,
            Err(EcashError::NotASigner)
        ));

        // ... but it is still one of the epoch whose credentials are doing the asking, and it
        // can still produce what they need
        state.ensure_signer_for_epoch(past_epoch).await?;
        let partial = state
            .partial_expiration_date_signatures(ecash_today_date(), past_epoch)
            .await?;
        assert_eq!(partial.epoch_id, past_epoch);

        Ok(())
    }

    /// The archive only answers for epochs it actually holds. An api that never derived a key
    /// for the requested epoch - because it was not yet in the group, or lost the file - has to
    /// say so rather than sign with the wrong key and label the result with the wrong epoch.
    #[tokio::test]
    async fn partial_expiration_date_signatures_are_refused_for_an_epoch_we_have_no_key_for(
    ) -> anyhow::Result<()> {
        let chain = SharedContractChain::new(1);
        initiate_dkg(&chain);

        cheap::run_ceremony(&chain, false);
        let past_epoch = chain.epoch().epoch_id;

        trigger_reset(&chain);
        cheap::run_ceremony(&chain, false);
        let current_epoch = chain.epoch().epoch_id;
        let current_keys = cheap::install_real_verification_keys(&chain);

        let me = chain.group_member_addresses()[0].clone();
        let state = contract_backed_ecash_state(&chain, me).await;

        // this api holds only the key it derived in the current ceremony
        state
            .local
            .ecash_keypair
            .set(KeyPairWithEpoch::new(
                current_keys.keypairs.into_iter().next().unwrap(),
                current_epoch,
            ))
            .await;
        state.local.ecash_keypair.validate();

        let expiration_date = ecash_today_date();

        // nothing stored for the past epoch, and no key to sign it with
        let refused = state
            .partial_expiration_date_signatures(expiration_date, past_epoch)
            .await;
        assert!(
            matches!(
                refused,
                Err(EcashError::InvalidSigningKeyEpoch {
                    requested,
                    available
                }) if requested == past_epoch && available == current_epoch
            ),
            "signed for an epoch whose key this api does not hold"
        );

        // the current epoch is of course still served
        let issued = state
            .partial_expiration_date_signatures(expiration_date, current_epoch)
            .await?;
        assert_eq!(issued.epoch_id, current_epoch);

        Ok(())
    }
}
