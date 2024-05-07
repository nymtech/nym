// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg;
use crate::ecash::dkg::controller::keys::archive_coconut_keypair;
use crate::ecash::dkg::controller::DkgController;
use crate::ecash::error::CoconutError;
use crate::ecash::keys::KeyPairWithEpoch;
use log::debug;
use nym_coconut_dkg_common::dealing::{chunk_dealing, DealingChunkInfo, MAX_DEALING_CHUNK_SIZE};
use nym_coconut_dkg_common::types::{DealingIndex, EpochId};
use nym_dkg::{Dealing, Scalar};
use rand::{CryptoRng, RngCore};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use thiserror::Error;

enum DealingGeneration {
    Fresh { number: u32 },
    Resharing { prior_secrets: Vec<Scalar> },
}

impl Debug for DealingGeneration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DealingGeneration::Fresh { number } => f
                .debug_struct("DealingGeneration::Fresh")
                .field("number", number)
                .finish(),
            DealingGeneration::Resharing { prior_secrets } => f
                .debug_struct("DealingGeneration::Resharing")
                .field("number", &prior_secrets.len())
                .finish(),
        }
    }
}

#[derive(Debug, Error)]
pub enum DealingGenerationError {
    #[error(transparent)]
    CoconutError(#[from] CoconutError),

    #[error("can't complete dealing exchange without registering public keys")]
    IncompletePublicKeyRegistration,

    #[error("contract state failure - the DKG threshold is unavailable even though dealing exchange has been initiated")]
    UnavailableContractThreshold,

    #[error("could not establish receiver index for epoch {epoch_id} even though we're a dealer!")]
    UnavailableReceiverIndex { epoch_id: EpochId },

    #[error("failed to archive coconut key for epoch {epoch_id} using path {}: {source}", path.display())]
    KeyArchiveFailure {
        epoch_id: EpochId,
        path: PathBuf,

        // I hate that we're using anyhow error source here, but changing that would require bigger refactoring
        #[source]
        source: anyhow::Error,
    },
}

impl<R: RngCore + CryptoRng> DkgController<R> {
    const DEALING_CHUNK_SIZE: usize = MAX_DEALING_CHUNK_SIZE;

    async fn generate_dealings(
        &mut self,
        epoch_id: EpochId,
        spec: DealingGeneration,
    ) -> Result<HashMap<DealingIndex, Dealing>, DealingGenerationError> {
        let threshold = self.dkg_client.get_current_epoch_threshold().await?.ok_or(
            CoconutError::UnrecoverableState {
                reason: String::from("Threshold should have been set"),
            },
        )?;

        let dealer_index = self
            .state
            .registration_state(epoch_id)?
            .assigned_index
            .ok_or(CoconutError::UnrecoverableState {
                reason: String::from("Node index should have been set"),
            })?;

        // in our case every dealer is also a receiver

        // ASSUMPTION: all dealers see the same contract data, i.e. if one fails to decode and verify the receiver's key,
        // all of them will
        let filtered_receivers = self.state.valid_epoch_receivers_keys(epoch_id)?;

        let dbg_receivers = filtered_receivers.keys().collect::<Vec<_>>();
        debug!("generating dealings with threshold {threshold} for receivers: {dbg_receivers:?} with the following spec: {spec:?}. Our index is {dealer_index}");

        let mut dealings = HashMap::new();
        match spec {
            DealingGeneration::Fresh { number } => {
                for i in 0..number {
                    let dealing = Dealing::create(
                        &mut self.rng,
                        dkg::params(),
                        dealer_index,
                        threshold,
                        &filtered_receivers,
                        None,
                    );
                    dealings.insert(i as DealingIndex, dealing.0);
                }
            }
            DealingGeneration::Resharing { prior_secrets } => {
                for (i, secret) in prior_secrets.into_iter().enumerate() {
                    let dealing = Dealing::create(
                        &mut self.rng,
                        dkg::params(),
                        dealer_index,
                        threshold,
                        &filtered_receivers,
                        Some(secret),
                    );
                    dealings.insert(i as DealingIndex, dealing.0);
                }
            }
        }

        // update the state with the dealing information
        self.state
            .dealing_exchange_state_mut(epoch_id)?
            .generated_dealings
            .clone_from(&dealings);

        Ok(dealings)
    }

    async fn generate_fresh_dealings(
        &mut self,
        epoch_id: EpochId,
        number: u32,
    ) -> Result<HashMap<DealingIndex, Dealing>, DealingGenerationError> {
        self.generate_dealings(epoch_id, DealingGeneration::Fresh { number })
            .await
    }

    async fn generate_resharing_dealings(
        &mut self,
        epoch_id: EpochId,
        prior_secrets: Vec<Scalar>,
    ) -> Result<HashMap<DealingIndex, Dealing>, DealingGenerationError> {
        self.generate_dealings(epoch_id, DealingGeneration::Resharing { prior_secrets })
            .await
    }

    async fn resubmit_pregenerated_dealings(
        &self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DealingGenerationError> {
        let dealing_state = self.state.dealing_exchange_state(epoch_id)?;
        let address = self.dkg_client.get_address().await.to_string();

        let status = self
            .dkg_client
            .get_dealings_statuses(epoch_id, address)
            .await?;

        if status.all_dealings_fully_submitted {
            warn!("we have actually submitted all dealings for epoch {epoch_id}, but somehow haven't finalized the state!");
            return Ok(());
        }

        // check which dealing is actually present on the chain (some might have gotten stuck in the mempool for quite a while)
        for (dealing_index, dealing) in &dealing_state.generated_dealings {
            // check the dealing
            let Some(dealing_status) = status.dealing_submission_status.get(dealing_index) else {
                // we should NEVER see this error
                error!("we have generated a dealing for index {dealing_index} but the contract does not require its submission!");
                continue;
            };

            if !dealing_status.has_metadata {
                // if the metadata doesn't exist on the chain, we haven't submitted anything - treat it as fresh submission
                self.submit_fresh_dealing(*dealing_index, dealing, resharing)
                    .await?;
                continue;
            }

            if dealing_status.fully_submitted {
                warn!("we have already submitted the full dealing {dealing_index} before - we probably crashed or the chain timed out!");
                continue;
            }

            let mut needs_resubmission = HashSet::new();
            for (chunk_id, chunk_status) in &dealing_status.chunk_submission_status {
                if !chunk_status.submitted() {
                    needs_resubmission.insert(chunk_id);
                } else {
                    warn!("[dealing {dealing_index}]: we have already submitted chunk at index {chunk_id} before - we probably crashed or the chain timed out!");
                }
            }

            warn!("[dealing {dealing_index}]: the following chunks need to be resubmitted: {needs_resubmission:?}");

            // perform the chunking (again)
            let mut chunks =
                chunk_dealing(*dealing_index, dealing.to_bytes(), Self::DEALING_CHUNK_SIZE);
            for chunk_index in needs_resubmission {
                // this is a hard failure, panic level, actually.
                // because we have already committed to dealings of particular size
                // yet we don't have relevant chunks after chunking
                let chunk = chunks
                    .remove(chunk_index)
                    .expect("chunking specification has changed mid-exchange!");
                debug!("[dealing {dealing_index}]: resubmitting chunk index {chunk_index}");
                self.dkg_client.submit_dealing_chunk(chunk).await?;
            }
        }
        Ok(())
    }

    async fn submit_fresh_dealing(
        &self,
        dealing_index: DealingIndex,
        dealing: &Dealing,
        resharing: bool,
    ) -> Result<(), DealingGenerationError> {
        let bytes = dealing.to_bytes();

        // construct metadata
        let chunk_info = DealingChunkInfo::construct(bytes.len(), Self::DEALING_CHUNK_SIZE);

        let total_chunks = chunk_info.len();
        debug!("dealing at index {dealing_index} has been chunked into {total_chunks} pieces",);

        // submit the metadata
        self.dkg_client
            .submit_dealing_metadata(dealing_index, chunk_info, resharing)
            .await?;

        // actually chunk the dealing and submit the chunks
        let chunks = chunk_dealing(dealing_index, bytes, Self::DEALING_CHUNK_SIZE);

        for (chunk_index, chunk) in chunks {
            let human_index = chunk_index + 1;
            debug!("[dealing {dealing_index}]: submitting chunk index {chunk_index} ({human_index}/{total_chunks})");

            self.dkg_client.submit_dealing_chunk(chunk).await?;
        }

        Ok(())
    }

    /// Check whether this dealer can participate in the resharing
    /// by looking into the contract and ensuring it's been a dealer in the previous epoch
    async fn can_reshare(&self, epoch_id: EpochId) -> Result<bool, DealingGenerationError> {
        // SAFETY:
        // it's impossible for the contract to trigger resharing for the 0th epoch
        // otherwise some serious invariants have been broken
        #[allow(clippy::expect_used)]
        let previous_epoch_id = epoch_id
            .checked_sub(1)
            .expect("resharing epoch invariant has been broken");

        let address = self.dkg_client.get_address().await;
        Ok(self
            .dkg_client
            .dealer_in_epoch(previous_epoch_id, address.to_string())
            .await?)
    }

    /// Deal with the dealing generation case where the system requests resharing
    /// and this node contains an already derived coconut keypair from some previous epoch.
    async fn handle_resharing_with_prior_key(
        &mut self,
        epoch_id: EpochId,
        expected_key_size: u32,
        old_keypair: KeyPairWithEpoch,
    ) -> Result<(), DealingGenerationError> {
        // make sure we're allowed to participate in resharing
        if !self.can_reshare(epoch_id).await? {
            // we have to wait for other dealers to give us the dealings (hopefully)
            warn!("we we have an existing coconut keypair, but we're not allowed to participate in resharing");
            return Ok(());
        }

        // EDGE CASE:
        // make sure our keypair is from strictly the previous epoch
        // because our node might have been offline for multiple epochs and while we do have a coconut keypair,
        // it could be outdated and we can't use it for resharing
        let previous = epoch_id.saturating_sub(1);
        if old_keypair.issued_for_epoch != previous {
            warn!("our existing coconut keypair has been generated for a distant epoch ({} vs expected {previous} for resharing)", old_keypair.issued_for_epoch);
            // don't participate in resharing
            return Ok(());
        }

        // EDGE CASE:
        // we have changed the key size (because we wanted to add new attribute to credentials)
        // in this instance we can't reuse our key and have to generate brand new dealings
        if expected_key_size != 1 + old_keypair.keys.secret_key().size() as u32 {
            warn!("our existing coconut keypair has different size than the currently expected value ({expected_key_size} vs {})", old_keypair.keys.secret_key().size() as u32);
            self.generate_fresh_dealings(epoch_id, expected_key_size)
                .await?;
            return Ok(());
        }

        // generate resharing dealings
        let prior_secrets = old_keypair.hazmat_into_secrets();
        // safety:
        // the prior secrets will be immediately converted into `Polynomial` with the specified coefficient
        // that does implement `ZeroizeOnDrop`
        self.generate_resharing_dealings(epoch_id, prior_secrets)
            .await?;

        Ok(())
    }

    /// Second step of the DKG process during which the nym api will generate appropriate [Dealing] for
    /// other parties as indicated by public key registration from the previous step.
    ///
    /// Before submitting any dealings to the system, the node will persist them locally so that if any failure
    /// occurs, it will be possible to recover.
    pub(crate) async fn dealing_exchange(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), DealingGenerationError> {
        let dealing_state = self.state.dealing_exchange_state(epoch_id)?;

        // check if we have already submitted the dealings
        if dealing_state.completed() {
            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!("we have already submitted all the dealings for this epoch");
            return Ok(());
        }

        if !self.state.registration_state(epoch_id)?.completed() {
            return Err(DealingGenerationError::IncompletePublicKeyRegistration);
        }

        // FAILURE CASE:
        // check if we have already generated the dealings, but they failed to get sent to the contract for whatever reason
        if !dealing_state.generated_dealings.is_empty() {
            debug!("we have already generated the dealings for this epoch");
            self.resubmit_pregenerated_dealings(epoch_id, resharing)
                .await?;

            // if we managed to resubmit the dealings (i.e. we didn't return an error),
            // it means the state is complete now.
            info!("DKG: resubmitted previously generated dealings - finished dealing exchange");
            self.state.dealing_exchange_state_mut(epoch_id)?.completed = true;
            return Ok(());
        }

        // we don't have any prior information - grab, parse and cache it since we will need it in next steps
        // and it's not going to change during the epoch
        let dealers = self.dkg_client.get_current_dealers().await?;

        // EDGE CASE:
        // if there are no receivers(dealers) in this epoch for some reason,
        // don't attempt to generate dealings as this will fail with a panic
        if dealers.is_empty() {
            warn!("there are no active dealers/receivers to generate dealings for");
            self.state.dealing_exchange_state_mut(epoch_id)?.completed = true;
            return Ok(());
        }

        self.state.dkg_state_mut(epoch_id)?.set_dealers(dealers);

        // obtain our dealer index to correctly set receiver index (used for share decryption)
        let dealer_index = self.state.assigned_index(epoch_id)?;

        // update internally used threshold value which should have been available after all dealers registered
        let Some(threshold) = self.dkg_client.get_current_epoch_threshold().await? else {
            // if we're in the dealing exchange phase, the threshold must have been already established
            return Err(DealingGenerationError::UnavailableContractThreshold);
        };
        self.state
            .key_derivation_state_mut(epoch_id)?
            .expected_threshold = Some(threshold);

        // establish our receiver index
        let sorted_dealers = &self.state.dkg_state(epoch_id)?.dealing_exchange.dealers;
        let Some(receiver_index) = sorted_dealers.keys().position(|idx| idx == &dealer_index)
        else {
            // this branch should be impossible as `dealing_exchange` should never be called unless we're actually a dealer
            error!("could not establish receiver index for epoch {epoch_id} even though we're a dealer!");
            return Err(DealingGenerationError::UnavailableReceiverIndex { epoch_id });
        };
        self.state
            .dealing_exchange_state_mut(epoch_id)?
            .receiver_index = Some(receiver_index);

        // get the expected key size which will determine the number of dealings we need to construct
        let contract_state = self.dkg_client.get_contract_state().await?;
        let expected_key_size = contract_state.key_size;

        // there are few cases to cover here based on the resharing status and presence of coconut keypair:
        // - resharing + we have a key => we should use the prior secrets for the resharing dealings generation
        // - resharing + we don't have a key => we are a new party that joined the existing setup and we have to wait for others to give us the shares
        // - no resharing + we have a key => whole DKG has been restarted (probably enough new parties joined / old parties left) to trigger it
        // - no resharing + we don't have a key => either as above (but we're a new party) or it's the very first instance of the DKG
        if let Some(old_keypair) = self.state.take_coconut_keypair().await {
            let keypair_epoch = old_keypair.issued_for_epoch;

            if resharing {
                debug!("resharing + prior key");
                self.handle_resharing_with_prior_key(epoch_id, expected_key_size, old_keypair)
                    .await?;
            } else {
                debug!("no resharing + prior key");
                self.generate_fresh_dealings(epoch_id, expected_key_size)
                    .await?;
            }

            // EDGE CASE:
            // make sure to persist the state after possibly generating the resharing dealings as we're going to be archiving the keypair
            // (so we won't be able to create resharing dealings again if we crashed since we won't be able to load the keys)
            self.state.persist()?;
            // archive the keypair
            if let Err(source) = archive_coconut_keypair(&self.coconut_key_path, keypair_epoch) {
                return Err(DealingGenerationError::KeyArchiveFailure {
                    epoch_id,
                    path: self.coconut_key_path.clone(),
                    source,
                });
            }
        } else {
            // sure, the if statements could be collapsed, but i prefer to explicitly repeat the block for readability
            if resharing {
                debug!("resharing + no prior key -> nothing to do");
                if self.can_reshare(epoch_id).await? {
                    warn!("this dealer was expected to participate in resharing but it doesn't have any prior keys to use");
                }
            } else {
                debug!("no resharing + no prior key");
                self.generate_fresh_dealings(epoch_id, expected_key_size)
                    .await?;
            }
        }

        let dealings = &self
            .state
            .dealing_exchange_state(epoch_id)?
            .generated_dealings;
        let total = dealings.len();

        // if we have generated any dealings persist the state in case we crash so that we would still have the data on hand
        // for resubmission upon getting back up
        if total > 0 {
            self.state.persist()?;
        }

        for (i, (&dealing_index, dealing)) in dealings.iter().enumerate() {
            let i = i + 1;
            debug!("submitting dealing index {dealing_index} ({i}/{total})");

            self.submit_fresh_dealing(dealing_index, dealing, resharing)
                .await?;
        }

        self.state.dealing_exchange_state_mut(epoch_id)?.completed = true;
        info!("DKG: Finished dealing exchange");
        Ok(())
    }
}

// NOTE: the following tests currently do NOT cover all cases
// I've (@JS) only updated old, existing, tests. nothing more
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ecash::dkg::state::registration::KeyRejectionReason;
    use crate::ecash::keys::KeyPair;
    use crate::ecash::tests::fixtures::{dealers_fixtures, test_rng, TestingDkgControllerBuilder};
    use crate::ecash::tests::helpers::unchecked_decode_bte_key;
    use nym_coconut_dkg_common::types::DealerRegistrationDetails;
    use nym_compact_ecash::ttp_keygen;
    use nym_dkg::bte::PublicKeyWithProof;

    #[tokio::test]
    async fn exchange_dealing() -> anyhow::Result<()> {
        let mut rng = test_rng([69u8; 32]);
        let dealers = dealers_fixtures(&mut rng, 4);
        let self_dealer = dealers[0].clone();

        let mut controller = TestingDkgControllerBuilder::default()
            .with_threshold(2)
            .with_dealers(dealers.clone())
            .with_as_dealer(self_dealer.clone())
            .build()
            .await;

        let epoch = controller.dkg_client.get_current_epoch().await?.epoch_id;
        let key_size = controller.dkg_client.get_contract_state().await?.key_size;

        // initial state
        assert!(controller
            .state
            .dealing_exchange_state(epoch)?
            .dealers
            .is_empty());
        assert!(controller
            .state
            .dealing_exchange_state(epoch)?
            .generated_dealings
            .is_empty());

        // exchange
        let res = controller.dealing_exchange(epoch, false).await;
        assert!(res.is_ok());

        let expected_dealers = dealers
            .iter()
            .map(|d| {
                (
                    d.assigned_index,
                    unchecked_decode_bte_key(&d.bte_public_key_with_proof),
                )
            })
            .collect::<Vec<_>>();
        let dealers = controller
            .state
            .dealing_exchange_state(epoch)?
            .dealers
            .values()
            .map(|p| (p.assigned_index, p.unwrap_key()))
            .collect::<Vec<_>>();

        let generated_dealings = controller
            .state
            .dealing_exchange_state(epoch)?
            .generated_dealings
            .clone();

        assert_eq!(expected_dealers, dealers);
        assert_eq!(key_size as usize, generated_dealings.len());

        // also make sure the fake chain state contains our dealings (since we submitted them)
        let chain_state = controller.chain_state.lock().unwrap();

        let submitted_dealings = chain_state
            .dkg_contract
            .dealings
            .get(&epoch)
            .unwrap()
            .get(self_dealer.address.as_str())
            .unwrap();

        for (dealing_index, submitted_info) in submitted_dealings {
            let dealing = Dealing::try_from_bytes(&submitted_info.unchecked_rebuild())?;

            assert_eq!(generated_dealings.get(dealing_index).unwrap(), &dealing)
        }

        Ok(())
    }

    #[tokio::test]
    async fn invalid_bte_proof_dealing_posted() -> anyhow::Result<()> {
        let mut rng = test_rng([69u8; 32]);
        let mut dealers = dealers_fixtures(&mut rng, 4);
        let self_dealer = dealers[0].clone();

        // malform key of one of the dealers, but in such a way that it still deserializes correctly
        let bad_dealer_addr = dealers[1].address.clone();
        let mut bytes = bs58::decode(&dealers[1].bte_public_key_with_proof).into_vec()?;
        let initial_byte = *bytes.last_mut().unwrap();
        loop {
            let last_byte = bytes.last_mut().unwrap();
            let (ret, _) = last_byte.overflowing_add(1);
            *last_byte = ret;
            // stop when we find that value, or if we do a full round trip of u8 values
            // and can't find one, in which case this test is invalid
            if PublicKeyWithProof::try_from_bytes(&bytes).is_ok() {
                break;
            }
            if ret == initial_byte {
                panic!("did not find a valid byte")
            }
        }
        dealers[1].bte_public_key_with_proof = bs58::encode(&bytes).into_string();

        let mut controller = TestingDkgControllerBuilder::default()
            .with_threshold(2)
            .with_dealers(dealers.clone())
            .with_as_dealer(self_dealer.clone())
            .build()
            .await;

        let epoch = controller.dkg_client.get_current_epoch().await?.epoch_id;

        // exchange
        let res = controller.dealing_exchange(epoch, false).await;
        assert!(res.is_ok());

        let bad_dealer = controller
            .state
            .dealing_exchange_state(epoch)?
            .dealers
            .values()
            .find(|d| d.address == bad_dealer_addr)
            .unwrap();

        assert_eq!(
            KeyRejectionReason::InvalidBTEPublicKey,
            bad_dealer.unwrap_rejection()
        );

        Ok(())
    }

    #[tokio::test]
    async fn resharing_outside_initial_set() -> anyhow::Result<()> {
        let mut rng = test_rng([69u8; 32]);
        let dealers = dealers_fixtures(&mut rng, 4);
        let self_dealer = dealers[0].clone();

        let epoch = 1;

        let mut keys = ttp_keygen(3, 4).unwrap();
        let coconut_keypair = KeyPair::new();
        coconut_keypair
            .set(KeyPairWithEpoch::new(keys.pop().unwrap(), epoch))
            .await;

        let mut controller = TestingDkgControllerBuilder::default()
            .with_threshold(3)
            .with_dealers(dealers.clone())
            .with_as_dealer(self_dealer.clone())
            .with_keypair(coconut_keypair)
            .with_initial_epoch_id(epoch)
            .build()
            .await;

        let res = controller.dealing_exchange(epoch, true).await;
        assert!(res.is_ok());

        let expected_dealers = dealers
            .iter()
            .map(|d| {
                (
                    d.assigned_index,
                    unchecked_decode_bte_key(&d.bte_public_key_with_proof),
                )
            })
            .collect::<Vec<_>>();
        let dealers = controller
            .state
            .dealing_exchange_state(epoch)?
            .dealers
            .values()
            .map(|p| (p.assigned_index, p.unwrap_key()))
            .collect::<Vec<_>>();

        let generated_dealings = controller
            .state
            .dealing_exchange_state(epoch)?
            .generated_dealings
            .clone();

        assert_eq!(expected_dealers, dealers);

        // no dealings submitted for the epoch, because we're not an initial dealer
        assert!(generated_dealings.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn resharing_inside_initial_set() -> anyhow::Result<()> {
        let mut rng = test_rng([69u8; 32]);
        let dealers = dealers_fixtures(&mut rng, 4);
        let self_dealer = dealers[0].clone();

        let epoch = 1;

        let mut keys = ttp_keygen(3, 4).unwrap();
        let coconut_keypair = KeyPair::new();
        coconut_keypair
            .set(KeyPairWithEpoch::new(keys.pop().unwrap(), epoch - 1))
            .await;

        let mut controller = TestingDkgControllerBuilder::default()
            .with_threshold(3)
            .with_dealers(dealers.clone())
            .with_as_dealer(self_dealer.clone())
            .with_keypair(coconut_keypair)
            .with_initial_epoch_id(epoch)
            .build()
            .await;

        let chain = controller.chain_state.clone();

        // TODO: put that functionality in the builder
        chain
            .lock()
            .unwrap()
            .dkg_contract
            .dealers
            .entry(epoch - 1)
            .or_default()
            .insert(
                self_dealer.address.to_string(),
                DealerRegistrationDetails {
                    bte_public_key_with_proof: self_dealer.bte_public_key_with_proof.clone(),
                    ed25519_identity: self_dealer.ed25519_identity.clone(),
                    announce_address: self_dealer.announce_address.clone(),
                },
            );

        let key_size = controller.dkg_client.get_contract_state().await?.key_size;

        let res = controller.dealing_exchange(epoch, true).await;
        assert!(res.is_ok());

        let expected_dealers = dealers
            .iter()
            .map(|d| {
                (
                    d.assigned_index,
                    unchecked_decode_bte_key(&d.bte_public_key_with_proof),
                )
            })
            .collect::<Vec<_>>();
        let dealers = controller
            .state
            .dealing_exchange_state(epoch)?
            .dealers
            .values()
            .map(|p| (p.assigned_index, p.unwrap_key()))
            .collect::<Vec<_>>();

        let generated_dealings = controller
            .state
            .dealing_exchange_state(epoch)?
            .generated_dealings
            .clone();

        assert_eq!(expected_dealers, dealers);

        // now we have dealings
        assert_eq!(key_size as usize, generated_dealings.len());

        Ok(())
    }
}
