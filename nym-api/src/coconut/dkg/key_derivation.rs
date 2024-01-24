// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg;
use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::dkg::controller::keys::persist_coconut_keypair;
use crate::coconut::dkg::controller::DkgController;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use crate::coconut::helpers::accepted_vote_err;
use crate::coconut::keys::KeyPairWithEpoch;
use crate::coconut::state::BANDWIDTH_CREDENTIAL_PARAMS;
use cosmwasm_std::Addr;
use cw3::{ProposalResponse, Status};
use log::debug;
use nym_coconut::tests::helpers::transpose_matrix;
use nym_coconut::{check_vk_pairing, Base58, KeyPair, SecretKey, VerificationKey};
use nym_coconut_dkg_common::event_attributes::DKG_PROPOSAL_ID;
use nym_coconut_dkg_common::types::{DealingIndex, EpochId, NodeIndex, PartialContractDealing};
use nym_coconut_dkg_common::verification_key::owner_from_cosmos_msgs;
use nym_coconut_interface::KeyPair as CoconutKeyPair;
use nym_dkg::bte::{decrypt_share, PublicKey};
use nym_dkg::error::DkgError;
use nym_dkg::{bte, combine_shares, try_recover_verification_keys, Dealing, Threshold};
use nym_pemstore::KeyPairPath;
use nym_validator_client::nyxd::bip32::secp256k1::elliptic_curve::group;
use nym_validator_client::nyxd::bip32::secp256k1::elliptic_curve::group::GroupEncoding;
use nym_validator_client::nyxd::cosmwasm_client::logs::find_attribute;
use rand::{CryptoRng, RngCore};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

// // Filter the dealers based on what dealing they posted (or not) in the contract
//
// // TODO: change the return type to make sure that:
// // - each entry has the same number of dealings
// // - dealer data is not duplicated
// // - each dealer has submitted all or nothing
// async fn deterministic_filter_dealers(
//     dkg_client: &DkgClient,
//     state: &mut State,
//     epoch_id: EpochId,
//     threshold: Threshold,
//     resharing: bool,
// ) -> Result<Vec<BTreeMap<NodeIndex, (Addr, Dealing)>>, CoconutError> {
//     // if we're in resharing mode, the contract itself will forbid submission of dealings from
//     // parties that were not "initial" dealers, so we don't have to worry about it
//
//     let mut dealings_maps = Vec::new();
//     let initial_dealers_by_addr = state.current_dealers_by_addr();
//     let initial_receivers = state.current_dealers_by_idx();
//     let initial_resharing_dealers = if resharing {
//         dkg_client
//             .get_initial_dealers()
//             .await?
//             .map(|d| d.initial_dealers)
//             .unwrap_or_default()
//     } else {
//         vec![]
//     };
//
//     let params = dkg::params();
//
//     // note: this is a temporary solution to replicate the behaviour of the old code so that I wouldn't need to
//     // fix the filtering in this PR, because the old code is quite buggy and misses few edge cases
//     let mut raw_dealings = HashMap::new();
//     for dealer in state.all_dealers().keys() {
//         let dealer_dealings = dkg_client
//             .get_dealings(epoch_id, dealer.to_string())
//             .await?;
//         for dealing in dealer_dealings {
//             let old_contract_dealing = raw_dealings.entry(dealing.index).or_insert(Vec::new());
//             old_contract_dealing.push((dealer.clone(), dealing.data))
//         }
//     }
//
//     // this is a temporary thing to reintroduce the bug to make sure tests still pass : )
//     // i will fix it properly in next PR
//     for dealing_index in 0..5 {
//         let dealings = raw_dealings.remove(&dealing_index).unwrap_or_default();
//         let dealings_map =
//             BTreeMap::from_iter(dealings.into_iter().filter_map(|(dealer, dealing)| {
//                 match Dealing::try_from(&dealing) {
//                     Ok(dealing) => {
//                         if dealing
//                             .verify(params, threshold, &initial_receivers, None)
//                             .is_err()
//                         {
//                             state.mark_bad_dealer(
//                                 &dealer,
//                                 ComplaintReason::DealingVerificationError,
//                             );
//                             None
//                         } else {
//                             initial_dealers_by_addr
//                                 .get(&dealer)
//                                 .map(|idx| (*idx, (dealer, dealing)))
//                         }
//                     }
//                     Err(_) => {
//                         state.mark_bad_dealer(&dealer, ComplaintReason::MalformedDealing);
//                         None
//                     }
//                 }
//             }));
//         dealings_maps.push(dealings_map);
//     }
//
//     //
//     //
//     //     for dealer in initial_dealers_by_addr.keys() {
//     //     let Some(dealer_index) = initial_dealers_by_addr.get(dealer) else {
//     //         warn!("could not obtain dealer index of {dealer}");
//     //         continue;
//     //     };
//     //
//     //     let dealer_dealings = dkg_client
//     //         .get_dealings(epoch_id, dealer.to_string())
//     //         .await?;
//     //
//     //     for contract_dealing in dealer_dealings {
//     //         match Dealing::try_from(&contract_dealing.data) {
//     //             // FIXME: bug: this doesn't check resharing
//     //             Ok(dealing) => {
//     //                 if let Err(err) = dealing.verify(params, threshold, &initial_receivers, None) {
//     //                     println!("dealing verification failure from {dealer}: {err}");
//     //                     state.mark_bad_dealer(dealer, ComplaintReason::DealingVerificationError);
//     //                 } else {
//     //                     let entry = dealings_maps
//     //                         .entry(contract_dealing.index)
//     //                         .or_insert(BTreeMap::new());
//     //                     entry.insert(*dealer_index, (dealer.clone(), dealing));
//     //                 }
//     //             }
//     //             Err(err) => {
//     //                 warn!("malformed dealing from {dealer}: {err}");
//     //                 state.mark_bad_dealer(dealer, ComplaintReason::MalformedDealing);
//     //             }
//     //         }
//     //     }
//     // }
//
//     for (addr, _) in initial_dealers_by_addr.iter() {
//         // in resharing mode, we don't commit dealings from dealers outside the initial set
//         if !resharing || initial_resharing_dealers.contains(addr) {
//             for dealings_map in dealings_maps.iter() {
//                 if !dealings_map.iter().any(|(_, (address, _))| address == addr) {
//                     state.mark_bad_dealer(addr, ComplaintReason::MissingDealing);
//                     break;
//                 }
//             }
//         }
//     }
//
//     Ok(dealings_maps)
// }
//
// fn derive_partial_keypair(
//     state: &mut State,
//     threshold: Threshold,
//     dealings_maps: Vec<BTreeMap<NodeIndex, (Addr, Dealing)>>,
// ) -> Result<KeyPair, CoconutError> {
//     let filtered_receivers_by_idx = state.current_dealers_by_idx();
//     let filtered_dealers_by_addr = state.current_dealers_by_addr();
//     let dk = state.dkg_keypair().private_key();
//     let node_index_value = state.receiver_index_value()?;
//     let mut scalars = vec![];
//     let mut recovered_vks = vec![];
//     for dealings_map in dealings_maps.into_iter() {
//         let (filtered_dealers, filtered_dealings): (Vec<_>, Vec<_>) = dealings_map
//             .into_iter()
//             .filter_map(|(idx, (addr, dealing))| {
//                 if filtered_dealers_by_addr.keys().any(|a| addr == *a) {
//                     Some((idx, dealing))
//                 } else {
//                     None
//                 }
//             })
//             .unzip();
//         debug!(
//             "Recovering verification keys from dealings of dealers {:?} with receivers {:?}",
//             filtered_dealers,
//             filtered_receivers_by_idx.keys().collect::<Vec<_>>()
//         );
//         let recovered = try_recover_verification_keys(
//             &filtered_dealings,
//             threshold,
//             &filtered_receivers_by_idx,
//         )?;
//         recovered_vks.push(recovered);
//
//         debug!("Decrypting shares");
//         let shares = filtered_dealings
//             .iter()
//             .map(|dealing| decrypt_share(dk, node_index_value, &dealing.ciphertexts, None))
//             .collect::<Result<_, _>>()?;
//         debug!("Combining shares into one secret");
//         let scalar = combine_shares(shares, &filtered_dealers)?;
//         scalars.push(scalar);
//     }
//     state.set_recovered_vks(recovered_vks);
//
//     let x = scalars.pop().ok_or(CoconutError::DkgError(
//         DkgError::NotEnoughDealingsAvailable {
//             available: 0,
//             required: 1,
//         },
//     ))?;
//     let sk = SecretKey::create_from_raw(x, scalars);
//     let vk = sk.verification_key(&BANDWIDTH_CREDENTIAL_PARAMS);
//
//     Ok(CoconutKeyPair::from_keys(sk, vk))
// }

impl<R: RngCore + CryptoRng> DkgController<R> {
    fn verified_dealer_dealings(
        &self,
        epoch_id: EpochId,
        dealer: Addr,
        epoch_receivers: &BTreeMap<NodeIndex, bte::PublicKey>,
        raw_dealings: Vec<PartialContractDealing>,
        prior_public_key: Option<VerificationKey>,
    ) -> Result<Vec<(DealingIndex, Dealing)>, CoconutError> {
        let threshold = self.state.threshold(epoch_id)?;

        // extract G2 elements from the old verification key of the dealer for checking the resharing dealings
        let prior_public_components = match prior_public_key {
            Some(vk) => {
                if vk.beta_g2().len() != raw_dealings.len().saturating_sub(1) {
                    todo!()
                }

                std::iter::once(Some(*vk.alpha()))
                    .chain(vk.beta_g2().iter().copied().map(Some))
                    .collect::<Vec<_>>()
            }
            None => vec![None; raw_dealings.len()],
        };

        let mut temp_verified = Vec::with_capacity(raw_dealings.len());
        // make sure ALL of them verify correctly, we can't have a situation where dealing 2 is valid but dealing 3 is not
        for (raw_dealing, prior_public) in raw_dealings
            .into_iter()
            .zip(prior_public_components.into_iter())
        {
            let index = raw_dealing.index;

            // recover the actual dealing from its submitted bytes representation
            let dealing = match Dealing::try_from_bytes(&raw_dealing.data) {
                Ok(dealing) => dealing,
                Err(err) => {
                    warn!("failed to recover dealing {index} from {dealer}: {err}",);
                    todo!("blacklist")
                }
            };

            // make sure the cryptographic material embedded inside is actually valid
            if let Err(err) =
                dealing.verify(dkg::params(), threshold, epoch_receivers, prior_public)
            {
                warn!("dealing {index} from {dealer} is invalid: {err}");
                todo!("blacklist")
            }

            temp_verified.push((index, dealing))
        }

        Ok(temp_verified)
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
    ) -> Result<BTreeMap<DealingIndex, BTreeMap<NodeIndex, Dealing>>, CoconutError> {
        let expected_key_size = self.dkg_client.get_contract_state().await?.key_size;

        let mut valid_dealings: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();

        // 1. for every valid dealer in this epoch, obtain its dealings
        for (dealer, dealer_index) in self.state.valid_epoch_receivers(epoch_id)? {
            // if we're in resharing mode, the contract itself will forbid submission of dealings from
            // parties that were not "initial" dealers, so we don't have to worry about it

            // TODO: introduce caching here in case we crash because those queries are EXPENSIVE
            let raw_dealings = self
                .dkg_client
                .get_dealings(epoch_id, dealer.to_string())
                .await?;

            if raw_dealings.is_empty() {
                // we might be in resharing mode and this was not in "initial" set
                todo!()
            }

            // no point in verifying any dealings if we don't have all of them
            if raw_dealings.len() != expected_key_size as usize {
                todo!("blacklist dealer - incomplete dealing exchange")
            }

            // TODO:
            // if this is resharing DKG, get the public key of this dealer from the previous epoch
            // and use it for dealing(s) verification
            let old_public_key = if resharing {
                // 1. check state from previous epoch
                // 2. if that failed, query the chain
                None
            } else {
                None
            };

            if let Ok(verified_dealings) = self.verified_dealer_dealings(
                epoch_id,
                dealer,
                epoch_receivers,
                raw_dealings,
                old_public_key,
            ) {
                // if we managed to verify ALL the dealings from this dealer, insert them into the map
                for (dealing_index, dealing) in verified_dealings {
                    valid_dealings
                        .entry(dealing_index)
                        .or_default()
                        .insert(dealer_index, dealing);
                }
            } else {
                todo!("blacklist dealer")
            }

            // if let Ok(verified_dealings) = self.verified_dealer_dealings(
            //     epoch_id,
            //     dealer,
            //     epoch_receivers,
            //     raw_dealings,
            //     old_public_key,
            // ) {
            //     // if we managed to verify ALL the dealings from this dealer, insert them into the map
            //     for (dealing_index, dealing) in verified_dealings {
            //         valid_dealings
            //             .entry(dealing_index)
            //             .or_default()
            //             .insert(dealer_index, dealing);
            //     }
            // } else {
            //     todo!("blacklist dealer")
            // }
        }

        Ok(valid_dealings)
    }

    fn derive_partial_keypair(
        &mut self,
        epoch_id: EpochId,
        epoch_receivers: BTreeMap<NodeIndex, PublicKey>,
        dealings: BTreeMap<DealingIndex, BTreeMap<NodeIndex, Dealing>>,
    ) -> Result<KeyPairWithEpoch, CoconutError> {
        debug!("attempting to derive coconut keypair for epoch {epoch_id}");

        let threshold = self.state.threshold(epoch_id)?;
        let receiver_index = self.state.receiver_index(epoch_id)?;

        // TODO: make sure that each receiver received its dealings

        // SAFETY:
        // we have ensured before calling this function that the dealings map is non-empty
        // and has exactly 'expected key size' number of entries;
        // furthermore each entry has the same number of sub-entries (ALL dealings from given node must be valid)
        //
        // SAFETY2:
        // dealing indexing starts from 0
        if dealings[&0].len() < threshold as usize {
            // make sure we have sufficient number of dealings to derive keys for the provided threshold
            todo!("fail - can't reach threshold")
        }

        let all_dealers = dealings[&0].keys().copied().collect::<Vec<_>>();

        let mut derived_x = None;
        let mut derived_secrets = Vec::new();

        let total = dealings.len();

        // for every part of the key
        for (dealing_index, dealings) in dealings {
            let dealings_vec = dealings.into_values().collect::<Vec<_>>();

            let human_index = dealing_index + 1;
            debug!("recovering part {human_index}/{total} of the keys");

            debug!("recovering the partial verification keys");
            let recovered =
                try_recover_verification_keys(&dealings_vec, threshold, &epoch_receivers)?;

            self.state
                .key_derivation_state_mut(epoch_id)?
                .derived_partials
                .insert(dealing_index, recovered);

            debug!("decrypting received shares");
            // for every received share of the key
            let mut shares = Vec::with_capacity(dealings_vec.len());
            for (i, dealing) in dealings_vec.into_iter().enumerate() {
                // attempt to decrypt our portion
                let dk = self.state.dkg_keypair().private_key();
                let share = match decrypt_share(dk, receiver_index, &dealing.ciphertexts, None) {
                    Ok(share) => share,
                    Err(err) => {
                        let node_index = all_dealers[i];
                        error!("failed to decrypt share {human_index}/{total} generated from dealer {node_index}: {err} - can't generate the full key");
                        todo!("do something about it")
                    }
                };
                shares.push(share)
            }

            debug!("combining the shares into part {human_index}/{total} of the epoch key");

            // SAFETY: combining shares can only fail if we have different number shares and indices
            // however, we returned an error if decryption of any share failed and thus we know those values must match
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
        let sk = SecretKey::create_from_raw(derived_x.unwrap(), derived_secrets);
        let derived_vk = sk.verification_key(&BANDWIDTH_CREDENTIAL_PARAMS);

        // TODO: make sure derived_vk matches recovered VK

        Ok(KeyPairWithEpoch {
            keys: CoconutKeyPair::from_keys(sk, derived_vk),
            issued_for_epoch: epoch_id,
        })
    }

    async fn submit_partial_verification_key(
        &mut self,
        key: &VerificationKey,
        resharing: bool,
    ) -> Result<u64, CoconutError> {
        debug!("submitting derived partial verification key to the contract");
        let res = self
            .dkg_client
            .submit_verification_key_share(key.to_bs58(), resharing)
            .await?;
        let hash = res.transaction_hash;
        let proposal_id = find_attribute(&res.logs, "wasm", DKG_PROPOSAL_ID)
            .ok_or(CoconutError::ProposalIdError {
                reason: String::from("proposal id not found"),
            })?
            .value
            .parse::<u64>()
            .map_err(|_| CoconutError::ProposalIdError {
                reason: String::from("proposal id could not be parsed to u64"),
            })?;
        debug!("Submitted own verification key share, proposal id {proposal_id} is attached to it. tx hash: {hash}");

        Ok(proposal_id)
    }

    /// Third step of the DKG process during which the nym api will generate its Coconut keypair
    /// with the [Dealing] received from other dealers. It will then submit its verification key
    /// to the system so that it could be validated by other participants.
    pub(crate) async fn verification_key_submission(
        &mut self,
        epoch_id: EpochId,
        resharing: bool,
    ) -> Result<(), CoconutError> {
        let key_generation_state = self.state.key_derivation_state(epoch_id)?;

        // check if we have already generated the new keys and submitted verification proposal
        if key_generation_state.completed() {
            // TODO: ASSERT: we have a VALID key
            // TODO: ASSERT we have a valid proposal id

            // the only way this could be a false positive is if the chain forked and blocks got reverted,
            // but I don't think we have to worry about that
            debug!(
                "we have already generated key for this epoch and submitted validation proposal"
            );
            return Ok(());
        }

        // TODO: make sure we have keys for that lol
        // if we have keys and we're still here it means validation failed blah blah

        // FAILURE CASE:
        // check if we have already sent the verification key transaction, but it timed out or got stuck in the mempool and
        // eventually got executed without us knowing about it, because it's illegal to recommit the key
        let maybe_share = self
            .dkg_client
            .get_verification_key_share_status(epoch_id)
            .await?;
        if maybe_share.is_some() {
            todo!("finalize, we're done")
            // TODO: recover proposal id with some weird queries
        }

        // ASSUMPTION:
        // all nym-apis would have filtered the dealers the same way since they'd have had the same data
        let epoch_receivers = self.state.valid_epoch_receivers_keys(epoch_id)?;

        let dealings = self
            .get_valid_dealings(&epoch_receivers, epoch_id, resharing)
            .await?;
        if dealings.is_empty() {
            todo!("not a failure per se but something along the lines: can't continue, won't continue")
        }

        let dbg_dealers = dealings[&0].keys().collect::<Vec<_>>();
        debug!("going to use dealings generated by {dbg_dealers:?}");

        let coconut_keypair = self.derive_partial_keypair(epoch_id, epoch_receivers, dealings)?;

        if let Err(err) = persist_coconut_keypair(&coconut_keypair, &self.coconut_key_path) {
            todo!()
        }

        let proposal_id = self
            .submit_partial_verification_key(coconut_keypair.keys.verification_key(), resharing)
            .await?;

        self.state.set_coconut_keypair(coconut_keypair).await;
        let derivation_state = self.state.key_derivation_state_mut(epoch_id)?;

        derivation_state.completed = true;
        derivation_state.proposal_id = Some(proposal_id);

        info!("DKG: Finished key derivation");
        Ok(())

        // TODO: set completed etc.

        // let dealings = self.dkg_client.get_dealings();

        // if state.coconut_keypair_is_some().await {
        //     debug!("Coconut keypair was set previously, nothing to do");
        //     return Ok(());
        // }
        //
        //
        // let threshold = state.threshold()?;
        // let dealings_maps =
        //     deterministic_filter_dealers(dkg_client, state, epoch_id, threshold, resharing).await?;
        // debug!(
        //     "Filtered dealers to {:?}",
        //     dealings_maps[0].keys().collect::<Vec<_>>()
        // );
        // let coconut_keypair = derive_partial_keypair(state, threshold, dealings_maps)?;
        // debug!("Derived own coconut keypair");
        // let vk_share = coconut_keypair.verification_key().to_bs58();
        // nym_pemstore::store_keypair(&coconut_keypair, keypair_path)?;
        // let res = dkg_client
        //     .submit_verification_key_share(vk_share, resharing)
        //     .await?;
        // let proposal_id = find_attribute(&res.logs, "wasm", DKG_PROPOSAL_ID)
        //     .ok_or(CoconutError::ProposalIdError {
        //         reason: String::from("proposal id not found"),
        //     })?
        //     .value
        //     .parse::<u64>()
        //     .map_err(|_| CoconutError::ProposalIdError {
        //         reason: String::from("proposal id could not be parsed to u64"),
        //     })?;
        // debug!(
        //     "Submitted own verification key share, proposal id {} is attached to it",
        //     proposal_id
        // );
        // state.set_proposal_id(proposal_id);
        // state.set_coconut_keypair(epoch_id, coconut_keypair).await;
        // info!("DKG: Submitted own verification key");
        //
        // Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::coconut::tests::helpers::{
        derive_keypairs, exchange_dealings, initialise_controllers, initialise_dkg,
        submit_public_keys,
    };

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_all_good() -> anyhow::Result<()> {
        let mut controllers = initialise_controllers(4);
        let chain = controllers[0].chain_state.clone();
        let expected = chain.lock().unwrap().dkg_contract_state.clone();

        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        todo!()
        //
        // for controller in controllers.iter_mut() {
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     assert_eq!(filtered.len(), contract_state.key_size as usize);
        //     for mapping in filtered.iter() {
        //         assert_eq!(mapping.len(), 4);
        //     }
        // }
        //
        // Ok(())
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_one_bad_dealing() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        // let contract_state = clients_and_states[0]
        //     .dkg_client
        //     .get_contract_state()
        //     .await
        //     .unwrap();
        //
        // // corrupt just one dealing
        // db.dealings_db
        //     .write()
        //     .unwrap()
        //     .entry(0)
        //     .and_modify(|epoch_dealings| {
        //         let validator_dealings = epoch_dealings
        //             .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
        //             .or_default();
        //         let mut last = validator_dealings.pop().unwrap();
        //         last.data.0.pop();
        //         validator_dealings.push(last);
        //     });
        //
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     assert_eq!(filtered.len(), contract_state.key_size as usize);
        //     let corrupted_status = controller
        //         .state
        //         .all_dealers()
        //         .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
        //         .unwrap()
        //         .as_ref()
        //         .unwrap_err();
        //     assert_eq!(*corrupted_status, ComplaintReason::MissingDealing);
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_resharing_filter_one_missing_dealing() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states(&db).await;
        // let contract_state = clients_and_states[0]
        //     .dkg_client
        //     .get_contract_state()
        //     .await
        //     .unwrap();
        //
        // // add all but the first dealing
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     controller.dealing_exchange(0, true).await.unwrap();
        // }
        //
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
        //         initial_dealers: vec![Addr::unchecked(TEST_VALIDATORS_ADDRESS[0])],
        //         initial_height: 1,
        //     });
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         true,
        //     )
        //     .await
        //     .unwrap();
        //     assert_eq!(filtered.len(), contract_state.key_size as usize);
        //     let corrupted_status = controller
        //         .state
        //         .all_dealers()
        //         .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
        //         .unwrap()
        //         .as_ref()
        //         .unwrap_err();
        //
        //     assert_eq!(*corrupted_status, ComplaintReason::MissingDealing);
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_resharing_filter_one_noninitial_missing_dealing() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states(&db).await;
        // let contract_state = clients_and_states[0]
        //     .dkg_client
        //     .get_contract_state()
        //     .await
        //     .unwrap();
        //
        // // add all but the first dealing
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     controller.dealing_exchange(0, true).await.unwrap();
        // }
        //
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
        //         initial_dealers: vec![],
        //         initial_height: 1,
        //     });
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         true,
        //     )
        //     .await
        //     .unwrap();
        //     assert_eq!(filtered.len(), contract_state.key_size as usize);
        //     assert!(controller
        //         .state
        //         .all_dealers()
        //         .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
        //         .unwrap()
        //         .as_ref()
        //         .is_ok(),);
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_all_bad_dealings() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        // let contract_state = clients_and_states[0]
        //     .dkg_client
        //     .get_contract_state()
        //     .await
        //     .unwrap();
        //
        // // corrupt all dealings of one address
        // db.dealings_db
        //     .write()
        //     .unwrap()
        //     .entry(0)
        //     .and_modify(|epoch_dealings| {
        //         let validator_dealings = epoch_dealings
        //             .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
        //             .or_default();
        //         validator_dealings.iter_mut().for_each(|dealing| {
        //             dealing.data.0.pop();
        //         });
        //     });
        //
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     assert_eq!(filtered.len(), contract_state.key_size as usize);
        //     for mapping in filtered.iter() {
        //         assert_eq!(mapping.len(), 3);
        //     }
        //     let corrupted_status = controller
        //         .state
        //         .all_dealers()
        //         .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
        //         .unwrap()
        //         .as_ref()
        //         .unwrap_err();
        //     assert_eq!(*corrupted_status, ComplaintReason::MissingDealing);
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_malformed_dealing() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        // let contract_state = clients_and_states[0]
        //     .dkg_client
        //     .get_contract_state()
        //     .await
        //     .unwrap();
        //
        // // corrupt just one dealing
        // db.dealings_db
        //     .write()
        //     .unwrap()
        //     .entry(0)
        //     .and_modify(|epoch_dealings| {
        //         let validator_dealings = epoch_dealings
        //             .get_mut(TEST_VALIDATORS_ADDRESS[0])
        //             .expect("no dealing");
        //         let mut last = validator_dealings.pop().unwrap();
        //         last.data.0.pop();
        //         validator_dealings.push(last);
        //     });
        //
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     // second filter will leave behind the bad dealer and surface why it was left out
        //     // in the first place
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     assert_eq!(filtered.len(), contract_state.key_size as usize);
        //     let corrupted_status = controller
        //         .state
        //         .all_dealers()
        //         .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
        //         .unwrap()
        //         .as_ref()
        //         .unwrap_err();
        //     assert_eq!(*corrupted_status, ComplaintReason::MalformedDealing);
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_dealing_verification_error() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        // let contract_state = clients_and_states[0]
        //     .dkg_client
        //     .get_contract_state()
        //     .await
        //     .unwrap();
        //
        // // corrupt just one dealing
        // db.dealings_db
        //     .write()
        //     .unwrap()
        //     .entry(0)
        //     .and_modify(|epoch_dealings| {
        //         let validator_dealings = epoch_dealings
        //             .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
        //             .or_default();
        //         let mut last = validator_dealings.pop().unwrap();
        //         let value = last.data.0.pop().unwrap();
        //         if value == 42 {
        //             last.data.0.push(43);
        //         } else {
        //             last.data.0.push(42);
        //         }
        //         validator_dealings.push(last);
        //     });
        //
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     // second filter will leave behind the bad dealer and surface why it was left out
        //     // in the first place
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     assert_eq!(filtered.len(), contract_state.key_size as usize);
        //     let corrupted_status = controller
        //         .state
        //         .all_dealers()
        //         .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
        //         .unwrap()
        //         .as_ref()
        //         .unwrap_err();
        //     assert_eq!(*corrupted_status, ComplaintReason::DealingVerificationError);
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn partial_keypair_derivation() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        // for controller in clients_and_states.iter_mut() {
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     assert!(derive_partial_keypair(&mut controller.state, 2, filtered).is_ok());
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn partial_keypair_derivation_with_threshold() {
        todo!()
        // let db = MockContractDb::new();
        // let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        //
        // // corrupt just one dealing
        // db.dealings_db
        //     .write()
        //     .unwrap()
        //     .entry(0)
        //     .and_modify(|epoch_dealings| {
        //         let validator_dealings = epoch_dealings
        //             .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
        //             .or_default();
        //         let mut last = validator_dealings.pop().unwrap();
        //         last.data.0.pop();
        //         validator_dealings.push(last);
        //     });
        //
        // for controller in clients_and_states.iter_mut().skip(1) {
        //     let filtered = deterministic_filter_dealers(
        //         &controller.dkg_client,
        //         &mut controller.state,
        //         0,
        //         2,
        //         false,
        //     )
        //     .await
        //     .unwrap();
        //     assert!(derive_partial_keypair(&mut controller.state, 2, filtered).is_ok());
        // }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn submit_verification_key() -> anyhow::Result<()> {
        let mut controllers = initialise_controllers(4);
        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_epoch.epoch_id;

        initialise_dkg(&mut controllers, false);
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;

        for controller in controllers.iter_mut() {
            let res = controller.verification_key_submission(epoch, false).await;
            assert!(res.is_ok());

            assert!(controller.state.key_derivation_state(epoch)?.completed);
            let keys = controller.state.take_coconut_keypair().await;
            assert!(keys.is_some());
            assert_eq!(keys.as_ref().unwrap().issued_for_epoch, epoch);
        }

        Ok(())
    }
}
