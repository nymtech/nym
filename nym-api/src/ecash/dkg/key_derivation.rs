// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg;
use crate::ecash::dkg::controller::keys::persist_coconut_keypair;
use crate::ecash::dkg::controller::DkgController;
use crate::ecash::dkg::state::key_derivation::{DealerRejectionReason, DerivationFailure};
use crate::ecash::error::CoconutError;
use crate::ecash::keys::KeyPairWithEpoch;
use cosmwasm_std::Addr;
use log::debug;
use nym_coconut_dkg_common::event_attributes::DKG_PROPOSAL_ID;
use nym_coconut_dkg_common::types::{DealingIndex, EpochId, NodeIndex};
use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::utils::check_vk_pairing;
use nym_compact_ecash::{ecash_group_parameters, Base58, KeyPairAuth, VerificationKeyAuth};
use nym_dkg::{
    bte::{self, decrypt_share},
    combine_shares, try_recover_verification_keys, Dealing,
};
use nym_validator_client::nyxd::cosmwasm_client::logs::{find_attribute, Log};
use nym_validator_client::nyxd::Hash;
use rand::{CryptoRng, RngCore};
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyDerivationError {
    #[error(transparent)]
    CoconutError(#[from] CoconutError),

    #[error("can't complete key derivation without dealing exchange")]
    IncompleteDealingExchange,

    #[error("the initial, zeroth, epoch is set to be in resharing mode - this is illegal and should have been impossible!")]
    ZerothEpochResharing,

    #[error("could not recover our own proposal id from the submitted share")]
    UnrecoverableProposalId,

    #[error("failed to persist the generated keys to disk: {source}")]
    KeyPersistenceFailure { source: anyhow::Error },

    #[error("the state file has been tampered with - key generation state is marked as complete, but proposal id is not set")]
    TamperedStateNoProposal,

    #[error("the state file has been tampered with - key generation state is marked as complete, but the key doesn't exist")]
    TamperedStateNoKeys,

    #[error("the state file has been tampered with - we're in the middle of DKG for epoch {current_epoch}, but we loaded keys for epoch {keys_epoch}")]
    TamperedStateWrongEpochKeys {
        current_epoch: EpochId,
        keys_epoch: EpochId,
    },

    #[error("did not derive partial key for ourselves (receiver: {receiver_index})")]
    NoSelfPartialKey { receiver_index: usize },

    #[error("did not find the proposal id attribute in the transaction events. looked for event '{event_type}' and attribute {attribute_key}' in tx {tx_hash}")]
    MissingProposalIdAttribute {
        tx_hash: Hash,
        event_type: String,
        attribute_key: String,
    },

    #[error("the retrieved proposal id ('{raw}') could not be parsed into a number")]
    UnparsableProposalId { raw: String },
}

impl<R: RngCore + CryptoRng> DkgController<R> {
    fn verified_dealer_dealings(
        &self,
        epoch_id: EpochId,
        dealer: &Addr,
        epoch_receivers: &BTreeMap<NodeIndex, bte::PublicKey>,
        raw_dealings: HashMap<DealingIndex, Vec<u8>>,
        prior_public_key: Option<VerificationKeyAuth>,
    ) -> Result<Result<Vec<(DealingIndex, Dealing)>, DealerRejectionReason>, KeyDerivationError>
    {
        let threshold = self.state.threshold(epoch_id)?;

        // extract G2 elements from the old verification key of the dealer for checking the resharing dealings
        let prior_public_components = match prior_public_key {
            Some(vk) => {
                if vk.beta_g2().len() != raw_dealings.len().saturating_sub(1) {
                    return Ok(Err(DealerRejectionReason::LastEpochKeyOfWrongSize {
                        key_size: vk.beta_g2().len() + 1,
                        expected: raw_dealings.len(),
                    }));
                }

                let mut prior = HashMap::new();
                prior.insert(0, *vk.alpha());
                for (i, beta) in vk.beta_g2().iter().enumerate() {
                    // element 1, 2, ...
                    prior.insert((i + 1) as DealingIndex, *beta);
                }

                Some(prior)
            }
            None => None,
        };

        let mut temp_verified = Vec::with_capacity(raw_dealings.len());
        // make sure ALL of them verify correctly, we can't have a situation where dealing 2 is valid but dealing 3 is not
        for (index, data) in raw_dealings {
            // recover the actual dealing from its submitted bytes representation
            let dealing = match Dealing::try_from_bytes(&data) {
                Ok(dealing) => dealing,
                Err(err) => {
                    warn!("failed to recover dealing {index} from {dealer}: {err}");
                    return Ok(Err(DealerRejectionReason::MalformedDealing {
                        index,
                        err_msg: err.to_string(),
                    }));
                }
            };

            let prior_public = prior_public_components
                .as_ref()
                .and_then(|p| p.get(&index).copied());

            // make sure the cryptographic material embedded inside is actually valid
            if let Err(err) =
                dealing.verify(dkg::params(), threshold, epoch_receivers, prior_public)
            {
                warn!("dealing {index} from {dealer} is invalid: {err}");
                return Ok(Err(DealerRejectionReason::InvalidDealing {
                    index,
                    err_msg: err.to_string(),
                }));
            }

            temp_verified.push((index, dealing))
        }

        Ok(Ok(temp_verified))
    }

    fn blacklist_dealer(
        &mut self,
        epoch_id: EpochId,
        dealer: Addr,
        reason: DealerRejectionReason,
    ) -> Result<(), KeyDerivationError> {
        self.state
            .key_derivation_state_mut(epoch_id)?
            .rejected_dealers
            .insert(dealer, reason);
        Ok(())
    }

    async fn get_old_verification_key(
        &self,
        epoch_id: EpochId,
        dealer: &Addr,
    ) -> Result<Option<VerificationKeyAuth>, KeyDerivationError> {
        let Some(previous_epoch) = epoch_id.checked_sub(1) else {
            return Err(KeyDerivationError::ZerothEpochResharing);
        };

        let Some(share) = self
            .dkg_client
            .get_verification_key_share(previous_epoch, dealer)
            .await?
        else {
            return Ok(None);
        };

        if !share.verified {
            return Ok(None);
        }

        // SAFETY:
        // since this share appears as 'verified' on the chain, it means the consensus of dealers confirmed its validity
        // and thus they must have been able to parse it, so the unwrap/expect here is fine
        Ok(Some(
            VerificationKeyAuth::try_from_bs58(&share.share)
                .expect("failed to deserialize VERIFIED key"),
        ))
    }

    async fn get_raw_dealings(
        &self,
        epoch_id: EpochId,
        dealer: &Addr,
        resharing: bool,
    ) -> Result<Result<HashMap<DealingIndex, Vec<u8>>, DealerRejectionReason>, KeyDerivationError>
    {
        let dealing_statuses = self
            .dkg_client
            .get_dealings_statuses(epoch_id, dealer.to_string())
            .await?;

        let submitted = dealing_statuses.full_dealings();

        // no point in making any queries if the dealer hasn't submitted ALL expected dealings
        if !dealing_statuses.all_dealings_fully_submitted {
            // the dealer only submitted some subset of dealings
            if submitted > 0 {
                return Ok(Err(
                    DealerRejectionReason::InsufficientNumberOfDealingsProvided {
                        got: submitted,
                        expected: dealing_statuses.dealing_submission_status.len(),
                    },
                ));
            }

            // if we're in the resharing mode and this dealer has not been a dealer in the previous epoch,
            // we don't expect to have received anything from them
            if resharing {
                // SAFETY:
                // it's impossible for the contract to trigger resharing for the 0th epoch
                // otherwise some serious invariants have been broken
                #[allow(clippy::expect_used)]
                let previous_epoch_id = epoch_id
                    .checked_sub(1)
                    .expect("resharing epoch invariant has been broken");
                if !self
                    .dkg_client
                    .dealer_in_epoch(previous_epoch_id, dealer.to_string())
                    .await?
                {
                    return Ok(Ok(HashMap::new()));
                }
            }

            return Ok(Err(DealerRejectionReason::NoDealingsProvided));
        }

        // TODO: introduce caching here in case we crash or chain times out because those queries are EXPENSIVE

        // rebuild the dealings
        let mut raw_dealings = HashMap::new();
        for (dealing_index, info) in dealing_statuses.dealing_submission_status {
            let mut raw_dealing = Vec::new();

            // note: we're iterating over a BTreeMap and so all chunk indices are guaranteed to be ordered
            for chunk_index in info.chunk_submission_status.into_keys() {
                let mut chunk_data = self
                    .dkg_client
                    .get_dealing_chunk(epoch_id, dealer.as_str(), dealing_index, chunk_index)
                    .await?;
                raw_dealing.append(&mut chunk_data);
            }

            raw_dealings.insert(dealing_index, raw_dealing);
        }

        Ok(Ok(raw_dealings))
    }

    /// Attempt to retrieve valid dealings submitted this epoch.
    ///
    /// For each dealer that submitted a valid public key, query its dealings.
    /// Then for each of those dealings, make sure they're cryptographically consistent
    pub(crate) async fn get_valid_dealings(
        &mut self,
        epoch_receivers: &BTreeMap<NodeIndex, bte::PublicKey>,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<BTreeMap<DealingIndex, BTreeMap<NodeIndex, Dealing>>, KeyDerivationError> {
        let mut valid_dealings: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();

        // given at MOST we'll have like 50 entries here, iterating over entire vector for lookup is fine

        // for every valid dealer in this epoch, obtain its dealings
        for (dealer, dealer_index) in self.state.valid_epoch_receivers(epoch_id)? {
            // note: if we're in resharing mode, the contract itself will forbid submission of dealings from
            // parties that were dealers in the previous epoch, so we don't have to worry about it

            let raw_dealings = match self.get_raw_dealings(epoch_id, &dealer, resharing).await? {
                Ok(dealings) => dealings,
                Err(rejection) => {
                    self.blacklist_dealer(epoch_id, dealer, rejection)?;
                    continue;
                }
            };

            // nothing to do
            if raw_dealings.is_empty() {
                continue;
            }

            // if this is resharing DKG, get the public key of this dealer from the previous epoch
            // and use it for dealing(s) verification
            let old_public_key = if resharing {
                // OPTIMIZATION:
                // rather than explicitly querying for the key, lookup the state from the previous epoch and reconstruct the key
                let Some(key) = self.get_old_verification_key(epoch_id, &dealer).await? else {
                    self.blacklist_dealer(
                        epoch_id,
                        dealer,
                        DealerRejectionReason::MissingVerifiedLastEpochKey,
                    )?;
                    continue;
                };
                Some(key)
            } else {
                None
            };

            // parse and validate the received dealings
            match self.verified_dealer_dealings(
                epoch_id,
                &dealer,
                epoch_receivers,
                raw_dealings,
                old_public_key,
            )? {
                Ok(verified_dealings) => {
                    // if we managed to verify ALL the dealings from this dealer, insert them into the map
                    for (dealing_index, dealing) in verified_dealings {
                        valid_dealings
                            .entry(dealing_index)
                            .or_default()
                            .insert(dealer_index, dealing);
                    }
                }
                Err(reason) => {
                    self.blacklist_dealer(epoch_id, dealer, reason)?;
                    continue;
                }
            }
        }

        Ok(valid_dealings)
    }

    fn derive_partial_keypair(
        &mut self,
        epoch_id: EpochId,
        epoch_receivers: BTreeMap<NodeIndex, bte::PublicKey>,
        dealings: BTreeMap<DealingIndex, BTreeMap<NodeIndex, Dealing>>,
    ) -> Result<Result<KeyPairWithEpoch, DerivationFailure>, KeyDerivationError> {
        debug!("attempting to derive coconut keypair for epoch {epoch_id}");

        let threshold = self.state.threshold(epoch_id)?;
        let receiver_index = self.state.receiver_index(epoch_id)?;

        // TODO: make sure that each receiver received its dealings

        // SAFETY:
        // we have ensured before calling this function that the dealings map is non-empty
        // and has exactly 'expected key size' number of entries;
        // furthermore each entry has the same number of sub-entries (ALL dealings from given dealer must be valid)
        //
        // SAFETY2:
        // dealing indexing starts from 0, so accessing 0th element is fine
        if dealings[&0].len() < threshold as usize {
            // make sure we have sufficient number of dealings to derive keys for the provided threshold,
            // otherwise we can't perform the lagrangian interpolation
            error!("we don't have enough dealings for key derivation");
            return Ok(Err(DerivationFailure::InsufficientNumberOfDealings {
                available: dealings[&0].len(),
                threshold,
            }));
        }

        let all_dealers = dealings[&0].keys().copied().collect::<Vec<_>>();

        let mut derived_x = None;
        let mut derived_secrets = Vec::new();

        let total = dealings.len();

        // for every part of the key
        for (dealing_index, dealings) in dealings {
            let human_index = dealing_index + 1;
            debug!("recovering part {human_index}/{total} of the keys");

            debug!("recovering the partial verification keys");
            let recovered =
                match try_recover_verification_keys(&dealings, threshold, &epoch_receivers) {
                    Ok(keys) => keys,
                    Err(err) => {
                        error!("failed to derive partial keys for index {dealing_index}: {err}");
                        return Ok(Err(DerivationFailure::KeyRecoveryFailure {
                            dealing_index,
                            err_msg: err.to_string(),
                        }));
                    }
                };

            self.state
                .key_derivation_state_mut(epoch_id)?
                .derived_partials
                .insert(dealing_index, recovered);

            debug!("decrypting received shares");

            // for every received share of the key
            let mut shares = Vec::with_capacity(dealings.len());
            for (dealer_index, dealing) in dealings.into_iter() {
                // attempt to decrypt our portion
                let dk = self.state.dkg_keypair().private_key();
                let share = match decrypt_share(dk, receiver_index, &dealing.ciphertexts, None) {
                    Ok(share) => share,
                    Err(err) => {
                        error!("failed to decrypt share {human_index}/{total} generated from dealer {dealer_index}: {err} - can't generate the full key");
                        return Ok(Err(DerivationFailure::ShareDecryptionFailure {
                            dealing_index,
                            dealer_index,
                            err_msg: err.to_string(),
                        }));
                    }
                };
                shares.push(share)
            }

            debug!("combining the shares into part {human_index}/{total} of the epoch key");

            // SAFETY: combining shares can only fail if we have different number shares and indices
            // however, we returned an explicit error if decryption of any share failed and thus we know those values must match
            let secret = combine_shares(shares, &all_dealers).unwrap();
            if derived_x.is_none() {
                derived_x = Some(secret)
            } else {
                derived_secrets.push(secret)
            }
        }

        // SAFETY:
        // we know we had a non-empty map of dealings and thus, at the very least, we must have derived a single secret
        // (i.e. the x-element)
        let sk = SecretKeyAuth::create_from_raw(derived_x.unwrap(), derived_secrets);
        let derived_vk = sk.verification_key();

        // make the key we derived out of the decrypted shares matches the partial key
        // (cryptographically there shouldn't be any reason for the mismatch,
        // but programmatically we might have accidentally used wrong index or something, so this is a good sanity check)
        let derived_partial = self
            .state
            .key_derivation_state(epoch_id)?
            .derived_partials_for(receiver_index)
            .ok_or(KeyDerivationError::NoSelfPartialKey { receiver_index })?;

        if !check_vk_pairing(ecash_group_parameters(), &derived_partial, &derived_vk) {
            // can't do anything, we got all dealings, we derived all keys, but somehow they don't match
            error!("our derived key does not match the expected partials!");
            return Ok(Err(DerivationFailure::MismatchedPartialKey));
        }

        Ok(Ok(KeyPairWithEpoch::new(
            KeyPairAuth::from_keys(sk, derived_vk),
            epoch_id,
        )))
    }

    async fn submit_partial_verification_key(
        &self,
        key: &VerificationKeyAuth,
        resharing: bool,
    ) -> Result<u64, KeyDerivationError> {
        fn extract_proposal_id_from_logs(
            logs: &[Log],
            tx_hash: Hash,
        ) -> Result<u64, KeyDerivationError> {
            let event_type = "wasm";
            let attribute_key = DKG_PROPOSAL_ID;
            let proposal_attribute = find_attribute(logs, event_type, attribute_key).ok_or(
                KeyDerivationError::MissingProposalIdAttribute {
                    tx_hash,
                    event_type: event_type.to_string(),
                    attribute_key: attribute_key.to_string(),
                },
            )?;

            proposal_attribute
                .value
                .parse()
                .map_err(|_| KeyDerivationError::UnparsableProposalId {
                    raw: proposal_attribute.value.clone(),
                })
        }

        debug!("submitting derived partial verification key to the contract");
        let res = self
            .dkg_client
            .submit_verification_key_share(key.to_bs58(), resharing)
            .await?;
        let hash = res.transaction_hash;
        let proposal_id = extract_proposal_id_from_logs(&res.logs, hash)?;
        debug!("Submitted own verification key share, proposal id {proposal_id} is attached to it. tx hash: {hash}");

        Ok(proposal_id)
    }

    async fn recover_proposal_id(&self) -> Result<u64, KeyDerivationError> {
        // unfortunately because the [dkg] contract doesn't store the proposal ids, we have to go through the list of ALL
        // submitted proposals and find the one with our address
        self.get_validation_proposals()
            .await?
            .get(self.dkg_client.get_address().await.as_ref())
            .copied()
            .ok_or(KeyDerivationError::UnrecoverableProposalId)
    }

    fn complete_with_proposal(
        &mut self,
        epoch_id: EpochId,
        proposal_id: u64,
    ) -> Result<(), KeyDerivationError> {
        let derivation_state = self.state.key_derivation_state_mut(epoch_id)?;
        derivation_state.completed = Some(Ok(()));
        derivation_state.proposal_id = Some(proposal_id);
        info!("DKG: Finished key derivation");

        Ok(())
    }

    fn complete_with_failure(
        &mut self,
        epoch_id: EpochId,
        failure: DerivationFailure,
    ) -> Result<(), KeyDerivationError> {
        let derivation_state = self.state.key_derivation_state_mut(epoch_id)?;
        error!("DKG: failed to finish the key derivation: {failure}");
        derivation_state.completed = Some(Err(failure));

        Ok(())
    }

    /// Check if we have already sent the verification transaction, but we failed to obtain valid proposal id in the previous iteration.
    async fn maybe_recover_proposal_id(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<u64>, KeyDerivationError> {
        let maybe_share = self
            .dkg_client
            .get_verification_own_key_share(epoch_id)
            .await?;

        // we DID send the transaction and the share is on the chain
        if maybe_share.is_some() {
            // note: we only ever send the verification key AFTER persisting our key,
            // so if the share is on the chain we MUST have the key
            debug_assert!(self.state.coconut_keypair_is_some().await);

            let proposal_id = self.recover_proposal_id().await?;
            return Ok(Some(proposal_id));
        }

        Ok(None)
    }

    /// Check if we already have a valid coconut key in the storage, if so, attempt to submit the partial verification key share
    /// to the contract and return the generated proposal id.
    async fn maybe_submit_already_generated_keys(
        &self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<Option<u64>, KeyDerivationError> {
        if let Some(keys) = self.state.unchecked_coconut_keypair().await.deref() {
            let keys_epoch = keys.issued_for_epoch;
            return if keys_epoch == epoch_id {
                debug!("we have already generated keys for this epoch but failed to send them to the contract");

                let proposal_id = self
                    .submit_partial_verification_key(&keys.keys.verification_key(), resharing)
                    .await?;
                Ok(Some(proposal_id))
            } else {
                error!("the state file has been tampered with - we're in the middle of DKG for epoch {epoch_id}, but we loaded keys for epoch {keys_epoch}");
                Err(KeyDerivationError::TamperedStateWrongEpochKeys {
                    current_epoch: epoch_id,
                    keys_epoch,
                })
            };
        };

        Ok(None)
    }

    /// Third step of the DKG process during which the nym api will generate its Coconut keypair
    /// with the [Dealing] received from other dealers. It will then submit its verification key
    /// to the system so that it could be validated by other participants.
    pub(crate) async fn verification_key_submission(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), KeyDerivationError> {
        let key_generation_state = self.state.key_derivation_state(epoch_id)?;

        // check if we have already generated the new keys and submitted verification proposal
        if key_generation_state.completed_with_success() {
            if key_generation_state.proposal_id.is_none() {
                error!("the state file has been tampered with - key generation state is marked as complete, but proposal id is not set");
                return Err(KeyDerivationError::TamperedStateNoProposal);
            }
            if !self.state.coconut_keypair_is_some().await {
                error!("the state file has been tampered with - key generation state is marked as complete, but the key doesn't exist");
                return Err(KeyDerivationError::TamperedStateNoKeys);
            }

            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!(
                "we have already generated key for this epoch and submitted validation proposal"
            );
            return Ok(());
        } else if let Some(failure) = key_generation_state.completion_failure() {
            error!("key derivation failed with unrecoverable failure: {failure}");
            return Ok(());
        }

        if !self.state.dealing_exchange_state(epoch_id)?.completed {
            return Err(KeyDerivationError::IncompleteDealingExchange);
        }

        // FAILURE CASE:
        // check if we have already sent the verification key transaction, but it timed out or got stuck in the mempool and
        // eventually got executed without us knowing about it, because it's illegal to recommit the key
        if let Some(proposal_id) = self.maybe_recover_proposal_id(epoch_id).await? {
            return self.complete_with_proposal(epoch_id, proposal_id);
        }

        // FAILURE CASE:
        // check if we have already generated the keys, but we didn't send the tx at all - maybe the internet connection
        // was momentarily down or something
        if let Some(proposal_id) = self
            .maybe_submit_already_generated_keys(epoch_id, resharing)
            .await?
        {
            return self.complete_with_proposal(epoch_id, proposal_id);
        }

        // ASSUMPTION:
        // all nym-apis would have filtered the dealers (receivers) the same way since they'd have had the same data
        let epoch_receivers = self.state.valid_epoch_receivers_keys(epoch_id)?;

        let dealings = self
            .get_valid_dealings(&epoch_receivers, epoch_id, resharing)
            .await?;
        if dealings.is_empty() {
            error!("did not recover ANY valid dealings - can't generate the epoch key");
            return self
                .complete_with_failure(epoch_id, DerivationFailure::NoValidDealings { epoch_id });
        }

        let dbg_dealers = dealings[&0].keys().collect::<Vec<_>>();
        debug!("going to use dealings generated by {dbg_dealers:?}");

        let coconut_keypair =
            match self.derive_partial_keypair(epoch_id, epoch_receivers, dealings)? {
                Ok(derived_keys) => derived_keys,
                Err(derivation_failure) => {
                    error!("we can't derive the coconut key: {derivation_failure}");
                    return self.complete_with_failure(epoch_id, derivation_failure);
                }
            };

        // before submitting our keys to the contract, persist the generated keypair
        if let Err(source) = persist_coconut_keypair(&coconut_keypair, &self.coconut_key_path) {
            return Err(KeyDerivationError::KeyPersistenceFailure { source });
        }

        let proposal_id = self
            .submit_partial_verification_key(&coconut_keypair.keys.verification_key(), resharing)
            .await?;

        self.state.set_coconut_keypair(coconut_keypair).await;
        self.complete_with_proposal(epoch_id, proposal_id)
    }
}

// NOTE: the following tests currently do NOT cover all cases
// I've (@JS) only updated old, existing, tests. nothing more
#[cfg(test)]
pub(crate) mod tests {
    use crate::ecash::dkg::state::key_derivation::DealerRejectionReason;
    use crate::ecash::tests::helpers::{
        exchange_dealings, initialise_controllers, initialise_dkg, submit_public_keys,
    };

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_all_good() -> anyhow::Result<()> {
        let validators = 3;

        let mut controllers = initialise_controllers(validators).await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        let key_size = chain.lock().unwrap().dkg_contract.contract_state.key_size;
        for controller in controllers.iter_mut() {
            let epoch_receivers = controller.state.valid_epoch_receivers_keys(epoch)?;

            let filtered = controller
                .get_valid_dealings(&epoch_receivers, epoch, false)
                .await?;

            assert_eq!(filtered.len(), key_size as usize);
            for dealing_map in filtered.values() {
                assert_eq!(dealing_map.len(), validators)
            }
            let corrupted_status = &controller
                .state
                .key_derivation_state(epoch)?
                .rejected_dealers;
            assert!(corrupted_status.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_one_bad_dealing() -> anyhow::Result<()> {
        let validators = 3;

        let mut controllers = initialise_controllers(validators).await;
        let address = controllers[0].cw_address().await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        let key_size = chain.lock().unwrap().dkg_contract.contract_state.key_size;

        // corrupt just one dealing
        chain
            .lock()
            .unwrap()
            .dkg_contract
            .dealings
            .entry(epoch)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings.get_mut(&address.to_string()).unwrap();
                let mut first = validator_dealings.remove(&0).unwrap();
                let first_chunk = first.chunks.get_mut(&0).unwrap();
                first_chunk.0.pop().unwrap();
                validator_dealings.insert(0, first);
            });

        for controller in controllers.iter_mut() {
            let epoch_receivers = controller.state.valid_epoch_receivers_keys(epoch)?;

            let filtered = controller
                .get_valid_dealings(&epoch_receivers, epoch, false)
                .await?;

            assert_eq!(filtered.len(), key_size as usize);
            let corrupted_status = controller
                .state
                .key_derivation_state(epoch)?
                .rejected_dealers
                .get(&address)
                .unwrap();
            assert!(matches!(
                corrupted_status,
                DealerRejectionReason::MalformedDealing { .. }
            ));
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_resharing_filter_one_missing_dealing() -> anyhow::Result<()> {
        let validators = 4;

        let mut controllers = initialise_controllers(validators).await;
        let address = controllers[0].cw_address().await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;
        let key_size = chain.lock().unwrap().dkg_contract.contract_state.key_size;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;

        // add all but the first dealing
        for controller in controllers.iter_mut().skip(1) {
            controller.dealing_exchange(epoch, false).await?;
        }

        for controller in controllers.iter_mut().skip(1) {
            let epoch_receivers = controller.state.valid_epoch_receivers_keys(epoch)?;

            let filtered = controller
                .get_valid_dealings(&epoch_receivers, epoch, false)
                .await?;

            assert_eq!(filtered.len(), key_size as usize);
            let corrupted_status = controller
                .state
                .key_derivation_state(epoch)?
                .rejected_dealers
                .get(&address)
                .unwrap();
            assert_eq!(corrupted_status, &DealerRejectionReason::NoDealingsProvided);
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_all_bad_dealings() -> anyhow::Result<()> {
        let validators = 3;

        let mut controllers = initialise_controllers(validators).await;
        let address = controllers[0].cw_address().await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        let key_size = chain.lock().unwrap().dkg_contract.contract_state.key_size;

        // // corrupt all dealings of one address
        chain
            .lock()
            .unwrap()
            .dkg_contract
            .dealings
            .entry(epoch)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings.get_mut(&address.to_string()).unwrap();
                validator_dealings.values_mut().for_each(|dealing| {
                    dealing.chunks.values_mut().for_each(|chunk| {
                        chunk.0.pop();
                    })
                });
            });

        for controller in controllers.iter_mut() {
            let epoch_receivers = controller.state.valid_epoch_receivers_keys(epoch)?;

            let filtered = controller
                .get_valid_dealings(&epoch_receivers, epoch, false)
                .await?;

            assert_eq!(filtered.len(), key_size as usize);
            for dealings in filtered.values() {
                assert_eq!(dealings.len(), validators - 1)
            }

            let corrupted_status = controller
                .state
                .key_derivation_state(epoch)?
                .rejected_dealers
                .get(&address)
                .unwrap();
            assert!(matches!(
                corrupted_status,
                DealerRejectionReason::MalformedDealing { .. }
            ));
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_dealing_verification_error() -> anyhow::Result<()> {
        let validators = 3;

        let mut controllers = initialise_controllers(validators).await;
        let address = controllers[0].cw_address().await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        let key_size = chain.lock().unwrap().dkg_contract.contract_state.key_size;

        // corrupt just one dealing
        chain
            .lock()
            .unwrap()
            .dkg_contract
            .dealings
            .entry(epoch)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings.get_mut(&address.to_string()).unwrap();
                let chunks = &mut validator_dealings.get_mut(&0).unwrap().chunks;
                let mut last_entry = chunks.last_entry().unwrap();
                let last = last_entry.get_mut();
                let value = last.0.pop().unwrap();
                if value == 42 {
                    last.0.push(43);
                } else {
                    last.0.push(42);
                }
            });

        for controller in controllers.iter_mut() {
            let epoch_receivers = controller.state.valid_epoch_receivers_keys(epoch)?;

            let filtered = controller
                .get_valid_dealings(&epoch_receivers, epoch, false)
                .await?;

            assert_eq!(filtered.len(), key_size as usize);
            let corrupted_status = controller
                .state
                .key_derivation_state(epoch)?
                .rejected_dealers
                .get(&address)
                .unwrap();
            assert!(matches!(
                corrupted_status,
                DealerRejectionReason::InvalidDealing { .. }
            ));
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn partial_keypair_derivation() -> anyhow::Result<()> {
        let validators = 3;

        let mut controllers = initialise_controllers(validators).await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        for controller in controllers.iter_mut() {
            let epoch_receivers = controller.state.valid_epoch_receivers_keys(epoch)?;

            let filtered = controller
                .get_valid_dealings(&epoch_receivers, epoch, false)
                .await?;

            let res = controller
                .derive_partial_keypair(epoch, epoch_receivers, filtered)
                .unwrap();
            assert!(res.is_ok());
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn partial_keypair_derivation_with_threshold() -> anyhow::Result<()> {
        let validators = 4;

        let mut controllers = initialise_controllers(validators).await;
        let address = controllers[0].cw_address().await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        // corrupt just one dealing
        chain
            .lock()
            .unwrap()
            .dkg_contract
            .dealings
            .entry(epoch)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings.get_mut(&address.to_string()).unwrap();
                let mut first = validator_dealings.remove(&0).unwrap();
                let first_chunk = first.chunks.get_mut(&0).unwrap();
                first_chunk.0.pop().unwrap();
                validator_dealings.insert(0, first);
            });

        for controller in controllers.iter_mut().skip(1) {
            let epoch_receivers = controller.state.valid_epoch_receivers_keys(epoch)?;
            let filtered = controller
                .get_valid_dealings(&epoch_receivers, epoch, false)
                .await?;

            let res = controller
                .derive_partial_keypair(epoch, epoch_receivers, filtered)
                .unwrap();
            assert!(res.is_ok());
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn submit_verification_key() -> anyhow::Result<()> {
        let validators = 4;
        let mut controllers = initialise_controllers(validators).await;
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        for controller in controllers.iter_mut() {
            let res = controller.verification_key_submission(epoch, false).await;
            assert!(res.is_ok());

            assert!(controller
                .state
                .key_derivation_state(epoch)?
                .completed_with_success());
            let keys = controller.state.take_coconut_keypair().await;
            assert!(keys.is_some());
            assert_eq!(keys.as_ref().unwrap().issued_for_epoch, epoch);
        }

        Ok(())
    }
}
