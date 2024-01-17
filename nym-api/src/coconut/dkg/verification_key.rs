// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg;
use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use crate::coconut::helpers::accepted_vote_err;
use crate::coconut::state::BANDWIDTH_CREDENTIAL_PARAMS;
use cosmwasm_std::Addr;
use cw3::{ProposalResponse, Status};
use log::debug;
use nym_coconut::tests::helpers::transpose_matrix;
use nym_coconut::{check_vk_pairing, Base58, KeyPair, SecretKey, VerificationKey};
use nym_coconut_dkg_common::event_attributes::DKG_PROPOSAL_ID;
use nym_coconut_dkg_common::types::{EpochId, NodeIndex};
use nym_coconut_dkg_common::verification_key::owner_from_cosmos_msgs;
use nym_coconut_interface::KeyPair as CoconutKeyPair;
use nym_dkg::bte::decrypt_share;
use nym_dkg::error::DkgError;
use nym_dkg::{combine_shares, try_recover_verification_keys, Dealing, Threshold};
use nym_pemstore::KeyPairPath;
use nym_validator_client::nyxd::cosmwasm_client::logs::find_attribute;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

// Filter the dealers based on what dealing they posted (or not) in the contract

// TODO: change the return type to make sure that:
// - each entry has the same number of dealings
// - dealer data is not duplicated
// - each dealer has submitted all or nothing
async fn deterministic_filter_dealers(
    dkg_client: &DkgClient,
    state: &mut State,
    epoch_id: EpochId,
    threshold: Threshold,
    resharing: bool,
) -> Result<Vec<BTreeMap<NodeIndex, (Addr, Dealing)>>, CoconutError> {
    let mut dealings_maps = Vec::new();
    let initial_dealers_by_addr = state.current_dealers_by_addr();
    let initial_receivers = state.current_dealers_by_idx();
    let initial_resharing_dealers = if resharing {
        dkg_client
            .get_initial_dealers()
            .await?
            .map(|d| d.initial_dealers)
            .unwrap_or_default()
    } else {
        vec![]
    };

    let params = dkg::params();

    // note: this is a temporary solution to replicate the behaviour of the old code so that I wouldn't need to
    // fix the filtering in this PR, because the old code is quite buggy and misses few edge cases
    let mut raw_dealings = HashMap::new();
    for dealer in state.all_dealers().keys() {
        let dealer_dealings = dkg_client
            .get_dealings(epoch_id, dealer.to_string())
            .await?;
        for dealing in dealer_dealings {
            let old_contract_dealing = raw_dealings.entry(dealing.index).or_insert(Vec::new());
            old_contract_dealing.push((dealer.clone(), dealing.data))
        }
    }

    // this is a temporary thing to reintroduce the bug to make sure tests still pass : )
    // i will fix it properly in next PR
    for dealing_index in 0..5 {
        let dealings = raw_dealings.remove(&dealing_index).unwrap_or_default();
        let dealings_map =
            BTreeMap::from_iter(dealings.into_iter().filter_map(|(dealer, dealing)| {
                match Dealing::try_from(&dealing) {
                    Ok(dealing) => {
                        if dealing
                            .verify(params, threshold, &initial_receivers, None)
                            .is_err()
                        {
                            state.mark_bad_dealer(
                                &dealer,
                                ComplaintReason::DealingVerificationError,
                            );
                            None
                        } else {
                            initial_dealers_by_addr
                                .get(&dealer)
                                .map(|idx| (*idx, (dealer, dealing)))
                        }
                    }
                    Err(_) => {
                        state.mark_bad_dealer(&dealer, ComplaintReason::MalformedDealing);
                        None
                    }
                }
            }));
        dealings_maps.push(dealings_map);
    }

    //
    //
    //     for dealer in initial_dealers_by_addr.keys() {
    //     let Some(dealer_index) = initial_dealers_by_addr.get(dealer) else {
    //         warn!("could not obtain dealer index of {dealer}");
    //         continue;
    //     };
    //
    //     let dealer_dealings = dkg_client
    //         .get_dealings(epoch_id, dealer.to_string())
    //         .await?;
    //
    //     for contract_dealing in dealer_dealings {
    //         match Dealing::try_from(&contract_dealing.data) {
    //             // FIXME: bug: this doesn't check resharing
    //             Ok(dealing) => {
    //                 if let Err(err) = dealing.verify(params, threshold, &initial_receivers, None) {
    //                     println!("dealing verification failure from {dealer}: {err}");
    //                     state.mark_bad_dealer(dealer, ComplaintReason::DealingVerificationError);
    //                 } else {
    //                     let entry = dealings_maps
    //                         .entry(contract_dealing.index)
    //                         .or_insert(BTreeMap::new());
    //                     entry.insert(*dealer_index, (dealer.clone(), dealing));
    //                 }
    //             }
    //             Err(err) => {
    //                 warn!("malformed dealing from {dealer}: {err}");
    //                 state.mark_bad_dealer(dealer, ComplaintReason::MalformedDealing);
    //             }
    //         }
    //     }
    // }

    for (addr, _) in initial_dealers_by_addr.iter() {
        // in resharing mode, we don't commit dealings from dealers outside the initial set
        if !resharing || initial_resharing_dealers.contains(addr) {
            for dealings_map in dealings_maps.iter() {
                if !dealings_map.iter().any(|(_, (address, _))| address == addr) {
                    state.mark_bad_dealer(addr, ComplaintReason::MissingDealing);
                    break;
                }
            }
        }
    }

    Ok(dealings_maps)
}

fn derive_partial_keypair(
    state: &mut State,
    threshold: Threshold,
    dealings_maps: Vec<BTreeMap<NodeIndex, (Addr, Dealing)>>,
) -> Result<KeyPair, CoconutError> {
    let filtered_receivers_by_idx = state.current_dealers_by_idx();
    let filtered_dealers_by_addr = state.current_dealers_by_addr();
    let dk = state.dkg_keypair().private_key();
    let node_index_value = state.receiver_index_value()?;
    let mut scalars = vec![];
    let mut recovered_vks = vec![];
    for dealings_map in dealings_maps.into_iter() {
        let (filtered_dealers, filtered_dealings): (Vec<_>, Vec<_>) = dealings_map
            .into_iter()
            .filter_map(|(idx, (addr, dealing))| {
                if filtered_dealers_by_addr.keys().any(|a| addr == *a) {
                    Some((idx, dealing))
                } else {
                    None
                }
            })
            .unzip();
        debug!(
            "Recovering verification keys from dealings of dealers {:?} with receivers {:?}",
            filtered_dealers,
            filtered_receivers_by_idx.keys().collect::<Vec<_>>()
        );
        let recovered = try_recover_verification_keys(
            &filtered_dealings,
            threshold,
            &filtered_receivers_by_idx,
        )?;
        recovered_vks.push(recovered);

        debug!("Decrypting shares");
        let shares = filtered_dealings
            .iter()
            .map(|dealing| decrypt_share(dk, node_index_value, &dealing.ciphertexts, None))
            .collect::<Result<_, _>>()?;
        debug!("Combining shares into one secret");
        let scalar = combine_shares(shares, &filtered_dealers)?;
        scalars.push(scalar);
    }
    state.set_recovered_vks(recovered_vks);

    let x = scalars.pop().ok_or(CoconutError::DkgError(
        DkgError::NotEnoughDealingsAvailable {
            available: 0,
            required: 1,
        },
    ))?;
    let sk = SecretKey::create_from_raw(x, scalars);
    let vk = sk.verification_key(&BANDWIDTH_CREDENTIAL_PARAMS);

    Ok(CoconutKeyPair::from_keys(sk, vk))
}

pub(crate) async fn verification_key_submission(
    dkg_client: &DkgClient,
    state: &mut State,
    epoch_id: EpochId,
    key_path: &PathBuf,
    resharing: bool,
) -> Result<(), CoconutError> {
    if state.coconut_keypair_is_some().await {
        debug!("Coconut keypair was set previously, nothing to do");
        return Ok(());
    }

    todo!()
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

fn validate_proposal(proposal: &ProposalResponse) -> Option<(Addr, u64)> {
    if proposal.status == Status::Open {
        if let Some(owner) = owner_from_cosmos_msgs(&proposal.msgs) {
            return Some((owner, proposal.id));
        }
    }
    None
}

pub(crate) async fn verification_key_validation(
    dkg_client: &DkgClient,
    state: &mut State,
    _resharing: bool,
) -> Result<(), CoconutError> {
    if state.voted_vks() {
        debug!("Already voted on the verification keys, nothing to do");
        return Ok(());
    }

    let epoch_id = dkg_client.get_current_epoch().await?.epoch_id;
    let vk_shares = dkg_client.get_verification_key_shares(epoch_id).await?;
    let proposal_ids = BTreeMap::from_iter(
        dkg_client
            .list_proposals()
            .await?
            .iter()
            .filter_map(validate_proposal),
    );
    let filtered_receivers_by_idx: Vec<_> =
        state.current_dealers_by_idx().keys().copied().collect();
    let recovered_partials: Vec<_> = state
        .recovered_vks()
        .iter()
        .map(|recovered_vk| recovered_vk.recovered_partials.clone())
        .collect();
    let recovered_partials = transpose_matrix(recovered_partials);
    let params = &BANDWIDTH_CREDENTIAL_PARAMS;
    for contract_share in vk_shares {
        if let Some(proposal_id) = proposal_ids.get(&contract_share.owner).copied() {
            match VerificationKey::try_from_bs58(contract_share.share) {
                Ok(vk) => {
                    if let Some(idx) = filtered_receivers_by_idx
                        .iter()
                        .position(|node_index| contract_share.node_index == *node_index)
                    {
                        let ret = if !check_vk_pairing(params, &recovered_partials[idx], &vk) {
                            debug!(
                                "Voting NO to proposal {} because of failed VK pairing",
                                proposal_id
                            );
                            dkg_client
                                .vote_verification_key_share(proposal_id, false)
                                .await
                        } else {
                            debug!("Voting YES to proposal {}", proposal_id);
                            dkg_client
                                .vote_verification_key_share(proposal_id, true)
                                .await
                        };
                        accepted_vote_err(ret)?;
                    }
                }
                Err(_) => {
                    debug!(
                        "Voting NO to proposal {} because of failed base 58 deserialization",
                        proposal_id
                    );
                    let ret = dkg_client
                        .vote_verification_key_share(proposal_id, false)
                        .await;
                    accepted_vote_err(ret)?;
                }
            }
        }
    }
    state.set_voted_vks();
    info!("DKG: Validated the other verification keys");
    Ok(())
}

pub(crate) async fn verification_key_finalization(
    dkg_client: &DkgClient,
    state: &mut State,
    _resharing: bool,
) -> Result<(), CoconutError> {
    if state.executed_proposal() {
        debug!("Already executed the proposal, nothing to do");
        return Ok(());
    }

    let proposal_id = state.proposal_id_value()?;
    dkg_client
        .execute_verification_key_share(proposal_id)
        .await?;
    state.set_executed_proposal();
    info!("DKG: Finalized own verification key on chain");

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::coconut::dkg::controller::DkgController;
    use crate::coconut::dkg::dealing::dealing_exchange;
    use crate::coconut::dkg::state::PersistentState;
    use crate::coconut::tests::DummyClient;
    use crate::coconut::KeyPair;
    use nym_coconut::aggregate_verification_keys;
    use nym_coconut_dkg_common::dealer::DealerDetails;
    use nym_coconut_dkg_common::types::{EpochId, InitialReplacementData, PartialContractDealing};
    use nym_coconut_dkg_common::verification_key::ContractVKShare;
    use nym_crypto::asymmetric::identity;
    use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
    use nym_validator_client::nyxd::AccountId;
    use rand::rngs::OsRng;
    use rand::Rng;
    use rand_07::thread_rng;
    use std::collections::HashMap;
    use std::env::temp_dir;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};
    use url::Url;

    struct MockContractDb {
        dealer_details_db: Arc<RwLock<HashMap<String, (DealerDetails, bool)>>>,
        // it's a really bad practice, but I'm not going to be changing it now...
        #[allow(clippy::type_complexity)]
        dealings_db: Arc<RwLock<HashMap<EpochId, HashMap<String, Vec<PartialContractDealing>>>>>,
        proposal_db: Arc<RwLock<HashMap<u64, ProposalResponse>>>,
        verification_share_db: Arc<RwLock<HashMap<String, ContractVKShare>>>,
        threshold_db: Arc<RwLock<Option<Threshold>>>,
        initial_dealers_db: Arc<RwLock<Option<InitialReplacementData>>>,
    }

    impl MockContractDb {
        pub fn new() -> Self {
            MockContractDb {
                dealer_details_db: Arc::new(Default::default()),
                dealings_db: Arc::new(Default::default()),
                proposal_db: Arc::new(Default::default()),
                verification_share_db: Arc::new(Default::default()),
                threshold_db: Arc::new(RwLock::new(Some(2))),
                initial_dealers_db: Arc::new(RwLock::new(Default::default())),
            }
        }
    }

    const TEST_VALIDATORS_ADDRESS: [&str; 4] = [
        "n1aq9kakfgwqcufr23lsv644apavcntrsqsk4yus",
        "n1s9l3xr4g0rglvk4yctktmck3h4eq0gp6z2e20v",
        "n19kl4py32vsk297dm93ezem992cdyzdy4zuc2x6",
        "n1jfrs6cmw9t7dv0x8cgny6geunzjh56n2s89fkv",
    ];

    async fn prepare_clients_and_states(db: &MockContractDb) -> Vec<DkgController> {
        let params = dkg::params();
        let mut clients_and_states = vec![];
        let identity_keypair = identity::KeyPair::new(&mut thread_rng());

        for addr in TEST_VALIDATORS_ADDRESS {
            let dkg_client = DkgClient::new(
                DummyClient::new(AccountId::from_str(addr).unwrap())
                    .with_dealer_details(&db.dealer_details_db)
                    .with_dealings(&db.dealings_db)
                    .with_proposal_db(&db.proposal_db)
                    .with_verification_share(&db.verification_share_db)
                    .with_threshold(&db.threshold_db)
                    .with_initial_dealers_db(&db.initial_dealers_db),
            );
            let keypair = DkgKeyPair::new(params, OsRng);
            let state = State::new(
                PathBuf::default(),
                PersistentState::default(),
                Url::parse("localhost:8000").unwrap(),
                keypair,
                *identity_keypair.public_key(),
                KeyPair::new(),
            );
            clients_and_states.push(DkgController::test_mock(dkg_client, state));
        }
        for controller in clients_and_states.iter_mut() {
            controller.public_key_submission(0, false).await.unwrap();
        }
        clients_and_states
    }

    async fn prepare_clients_and_states_with_dealing(db: &MockContractDb) -> Vec<DkgController> {
        let mut clients_and_states = prepare_clients_and_states(db).await;
        for controller in clients_and_states.iter_mut() {
            dealing_exchange(&controller.dkg_client, &mut controller.state, OsRng, false)
                .await
                .unwrap();
        }
        clients_and_states
    }

    async fn prepare_clients_and_states_with_submission(db: &MockContractDb) -> Vec<DkgController> {
        let mut clients_and_states = prepare_clients_and_states_with_dealing(db).await;
        for controller in clients_and_states.iter_mut() {
            let random_file: usize = OsRng.gen();
            let keypath = temp_dir().join(format!("coconut{}.pem", random_file));
            verification_key_submission(
                &controller.dkg_client,
                &mut controller.state,
                0,
                &keypath,
                false,
            )
            .await
            .unwrap();
            std::fs::remove_file(keypath).unwrap();
        }
        clients_and_states
    }

    async fn prepare_clients_and_states_with_validation(db: &MockContractDb) -> Vec<DkgController> {
        let mut clients_and_states = prepare_clients_and_states_with_submission(db).await;
        for controller in clients_and_states.iter_mut() {
            verification_key_validation(&controller.dkg_client, &mut controller.state, false)
                .await
                .unwrap();
        }
        clients_and_states
    }

    async fn prepare_clients_and_states_with_finalization(
        db: &MockContractDb,
    ) -> Vec<DkgController> {
        let mut clients_and_states = prepare_clients_and_states_with_validation(db).await;
        for controller in clients_and_states.iter_mut() {
            verification_key_finalization(&controller.dkg_client, &mut controller.state, false)
                .await
                .unwrap();
        }
        clients_and_states
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_all_good() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        let contract_state = clients_and_states[0]
            .dkg_client
            .get_contract_state()
            .await
            .unwrap();

        for controller in clients_and_states.iter_mut() {
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            assert_eq!(filtered.len(), contract_state.key_size as usize);
            for mapping in filtered.iter() {
                assert_eq!(mapping.len(), 4);
            }
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_one_bad_dealing() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        let contract_state = clients_and_states[0]
            .dkg_client
            .get_contract_state()
            .await
            .unwrap();

        // corrupt just one dealing
        db.dealings_db
            .write()
            .unwrap()
            .entry(0)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings
                    .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
                    .or_default();
                let mut last = validator_dealings.pop().unwrap();
                last.data.0.pop();
                validator_dealings.push(last);
            });

        for controller in clients_and_states.iter_mut().skip(1) {
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            assert_eq!(filtered.len(), contract_state.key_size as usize);
            let corrupted_status = controller
                .state
                .all_dealers()
                .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
                .unwrap()
                .as_ref()
                .unwrap_err();
            assert_eq!(*corrupted_status, ComplaintReason::MissingDealing);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_resharing_filter_one_missing_dealing() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states(&db).await;
        let contract_state = clients_and_states[0]
            .dkg_client
            .get_contract_state()
            .await
            .unwrap();

        // add all but the first dealing
        for controller in clients_and_states.iter_mut().skip(1) {
            dealing_exchange(&controller.dkg_client, &mut controller.state, OsRng, true)
                .await
                .unwrap();
        }

        for controller in clients_and_states.iter_mut().skip(1) {
            *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
                initial_dealers: vec![Addr::unchecked(TEST_VALIDATORS_ADDRESS[0])],
                initial_height: 1,
            });
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                true,
            )
            .await
            .unwrap();
            assert_eq!(filtered.len(), contract_state.key_size as usize);
            let corrupted_status = controller
                .state
                .all_dealers()
                .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
                .unwrap()
                .as_ref()
                .unwrap_err();

            assert_eq!(*corrupted_status, ComplaintReason::MissingDealing);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_resharing_filter_one_noninitial_missing_dealing() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states(&db).await;
        let contract_state = clients_and_states[0]
            .dkg_client
            .get_contract_state()
            .await
            .unwrap();

        // add all but the first dealing
        for controller in clients_and_states.iter_mut().skip(1) {
            dealing_exchange(&controller.dkg_client, &mut controller.state, OsRng, true)
                .await
                .unwrap();
        }

        for controller in clients_and_states.iter_mut().skip(1) {
            *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
                initial_dealers: vec![],
                initial_height: 1,
            });
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                true,
            )
            .await
            .unwrap();
            assert_eq!(filtered.len(), contract_state.key_size as usize);
            assert!(controller
                .state
                .all_dealers()
                .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
                .unwrap()
                .as_ref()
                .is_ok(),);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_all_bad_dealings() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        let contract_state = clients_and_states[0]
            .dkg_client
            .get_contract_state()
            .await
            .unwrap();

        // corrupt all dealings of one address
        db.dealings_db
            .write()
            .unwrap()
            .entry(0)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings
                    .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
                    .or_default();
                validator_dealings.iter_mut().for_each(|dealing| {
                    dealing.data.0.pop();
                });
            });

        for controller in clients_and_states.iter_mut().skip(1) {
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            assert_eq!(filtered.len(), contract_state.key_size as usize);
            for mapping in filtered.iter() {
                assert_eq!(mapping.len(), 3);
            }
            let corrupted_status = controller
                .state
                .all_dealers()
                .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
                .unwrap()
                .as_ref()
                .unwrap_err();
            assert_eq!(*corrupted_status, ComplaintReason::MissingDealing);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_malformed_dealing() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        let contract_state = clients_and_states[0]
            .dkg_client
            .get_contract_state()
            .await
            .unwrap();

        // corrupt just one dealing
        db.dealings_db
            .write()
            .unwrap()
            .entry(0)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings
                    .get_mut(TEST_VALIDATORS_ADDRESS[0])
                    .expect("no dealing");
                let mut last = validator_dealings.pop().unwrap();
                last.data.0.pop();
                validator_dealings.push(last);
            });

        for controller in clients_and_states.iter_mut().skip(1) {
            deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            // second filter will leave behind the bad dealer and surface why it was left out
            // in the first place
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            assert_eq!(filtered.len(), contract_state.key_size as usize);
            let corrupted_status = controller
                .state
                .all_dealers()
                .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
                .unwrap()
                .as_ref()
                .unwrap_err();
            assert_eq!(*corrupted_status, ComplaintReason::MalformedDealing);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_dealing_verification_error() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        let contract_state = clients_and_states[0]
            .dkg_client
            .get_contract_state()
            .await
            .unwrap();

        // corrupt just one dealing
        db.dealings_db
            .write()
            .unwrap()
            .entry(0)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings
                    .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
                    .or_default();
                let mut last = validator_dealings.pop().unwrap();
                let value = last.data.0.pop().unwrap();
                if value == 42 {
                    last.data.0.push(43);
                } else {
                    last.data.0.push(42);
                }
                validator_dealings.push(last);
            });

        for controller in clients_and_states.iter_mut().skip(1) {
            deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            // second filter will leave behind the bad dealer and surface why it was left out
            // in the first place
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            assert_eq!(filtered.len(), contract_state.key_size as usize);
            let corrupted_status = controller
                .state
                .all_dealers()
                .get(&Addr::unchecked(TEST_VALIDATORS_ADDRESS[0]))
                .unwrap()
                .as_ref()
                .unwrap_err();
            assert_eq!(*corrupted_status, ComplaintReason::DealingVerificationError);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn partial_keypair_derivation() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;
        for controller in clients_and_states.iter_mut() {
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            assert!(derive_partial_keypair(&mut controller.state, 2, filtered).is_ok());
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn partial_keypair_derivation_with_threshold() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;

        // corrupt just one dealing
        db.dealings_db
            .write()
            .unwrap()
            .entry(0)
            .and_modify(|epoch_dealings| {
                let validator_dealings = epoch_dealings
                    .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
                    .or_default();
                let mut last = validator_dealings.pop().unwrap();
                last.data.0.pop();
                validator_dealings.push(last);
            });

        for controller in clients_and_states.iter_mut().skip(1) {
            let filtered = deterministic_filter_dealers(
                &controller.dkg_client,
                &mut controller.state,
                0,
                2,
                false,
            )
            .await
            .unwrap();
            assert!(derive_partial_keypair(&mut controller.state, 2, filtered).is_ok());
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn submit_verification_key() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_submission(&db).await;

        for controller in clients_and_states.iter_mut() {
            assert!(db
                .proposal_db
                .read()
                .unwrap()
                .contains_key(&controller.state.proposal_id_value().unwrap()));
            assert!(controller.state.coconut_keypair_is_some().await);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_validation(&db).await;
        for controller in clients_and_states.iter_mut() {
            let proposal = db
                .proposal_db
                .read()
                .unwrap()
                .get(&controller.state.proposal_id_value().unwrap())
                .unwrap()
                .clone();
            assert_eq!(proposal.status, Status::Passed);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key_malformed_share() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_submission(&db).await;

        db.verification_share_db
            .write()
            .unwrap()
            .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
            .and_modify(|share| share.share.push('x'));

        for controller in clients_and_states.iter_mut() {
            verification_key_validation(&controller.dkg_client, &mut controller.state, false)
                .await
                .unwrap();
        }

        for (idx, controller) in clients_and_states.iter().enumerate() {
            let proposal = db
                .proposal_db
                .read()
                .unwrap()
                .get(&controller.state.proposal_id_value().unwrap())
                .unwrap()
                .clone();
            if idx == 0 {
                assert_eq!(proposal.status, Status::Rejected);
            } else {
                assert_eq!(proposal.status, Status::Passed);
            }
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key_unpaired_share() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_submission(&db).await;

        let second_share = db
            .verification_share_db
            .write()
            .unwrap()
            .get(TEST_VALIDATORS_ADDRESS[1])
            .unwrap()
            .share
            .clone();
        db.verification_share_db
            .write()
            .unwrap()
            .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
            .and_modify(|share| share.share = second_share);

        for controller in clients_and_states.iter_mut() {
            verification_key_validation(&controller.dkg_client, &mut controller.state, false)
                .await
                .unwrap();
        }

        for (idx, controller) in clients_and_states.iter().enumerate() {
            let proposal = db
                .proposal_db
                .read()
                .unwrap()
                .get(&controller.state.proposal_id_value().unwrap())
                .unwrap()
                .clone();
            if idx == 0 {
                assert_eq!(proposal.status, Status::Rejected);
            } else {
                assert_eq!(proposal.status, Status::Passed);
            }
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn finalize_verification_key() {
        let db = MockContractDb::new();
        let clients_and_states = prepare_clients_and_states_with_finalization(&db).await;

        for controller in clients_and_states.iter() {
            let proposal = db
                .proposal_db
                .read()
                .unwrap()
                .get(&controller.state.proposal_id_value().unwrap())
                .unwrap()
                .clone();
            assert_eq!(proposal.status, Status::Executed);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn reshare_preserves_keys() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_finalization(&db).await;
        for controller in clients_and_states.iter_mut() {
            controller.state.set_was_in_progress();
        }

        let mut vks = vec![];
        let mut indices = vec![];
        for controller in clients_and_states.iter() {
            let vk = controller
                .state
                .coconut_keypair()
                .await
                .as_ref()
                .unwrap()
                .keys
                .verification_key()
                .clone();
            let index = controller.state.node_index().unwrap();
            vks.push(vk);
            indices.push(index);
        }
        let initial_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();

        let new_dkg_client = DkgClient::new(
            DummyClient::new(
                AccountId::from_str("n1sqkxzh7nl6kgndr4ew9795t2nkwmd8tpql67q7").unwrap(),
            )
            .with_dealer_details(&db.dealer_details_db)
            .with_dealings(&db.dealings_db)
            .with_proposal_db(&db.proposal_db)
            .with_verification_share(&db.verification_share_db)
            .with_threshold(&db.threshold_db)
            .with_initial_dealers_db(&db.initial_dealers_db),
        );
        let keypair = DkgKeyPair::new(dkg::params(), OsRng);
        let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        let state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            keypair,
            *identity_keypair.public_key(),
            KeyPair::new(),
        );

        for (_, active) in db.dealer_details_db.write().unwrap().values_mut() {
            *active = false;
        }

        *db.dealings_db.write().unwrap() = Default::default();
        *db.verification_share_db.write().unwrap() = Default::default();
        let mut initial_dealers = vec![];
        for controller in clients_and_states.iter() {
            let client_address =
                Addr::unchecked(controller.dkg_client.get_address().await.as_ref());
            initial_dealers.push(client_address);
        }
        *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
            initial_dealers,
            initial_height: 1,
        });
        *clients_and_states.first_mut().unwrap() = DkgController::test_mock(new_dkg_client, state);

        for controller in clients_and_states.iter_mut() {
            controller.public_key_submission(0, true).await.unwrap();
        }

        for controller in clients_and_states.iter_mut() {
            dealing_exchange(&controller.dkg_client, &mut controller.state, OsRng, true)
                .await
                .unwrap();
        }

        for controller in clients_and_states.iter_mut() {
            let random_file: usize = OsRng.gen();
            let keypath = temp_dir().join(format!("coconut{}.pem", random_file));
            verification_key_submission(
                &controller.dkg_client,
                &mut controller.state,
                0,
                &keypath,
                true,
            )
            .await
            .unwrap();
            std::fs::remove_file(keypath).unwrap();
        }
        for controller in clients_and_states.iter_mut() {
            verification_key_validation(&controller.dkg_client, &mut controller.state, true)
                .await
                .unwrap();
        }
        for controller in clients_and_states.iter_mut() {
            verification_key_finalization(&controller.dkg_client, &mut controller.state, true)
                .await
                .unwrap();
        }
        assert!(db
            .proposal_db
            .read()
            .unwrap()
            .values()
            .all(|proposal| { proposal.status == Status::Executed }));

        let mut vks = vec![];
        let mut indices = vec![];
        for controller in clients_and_states.iter() {
            let vk = controller
                .state
                .coconut_keypair()
                .await
                .as_ref()
                .unwrap()
                .keys
                .verification_key()
                .clone();
            let index = controller.state.node_index().unwrap();
            vks.push(vk);
            indices.push(index);
        }
        let reshared_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();
        assert_eq!(initial_master_vk, reshared_master_vk);
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn reshare_after_reset() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_finalization(&db).await;
        for controller in clients_and_states.iter_mut() {
            controller.state.set_was_in_progress();
        }

        let new_dkg_client = DkgClient::new(
            DummyClient::new(
                AccountId::from_str("n1vxkywf9g4cg0k2dehanzwzz64jw782qm0kuynf").unwrap(),
            )
            .with_dealer_details(&db.dealer_details_db)
            .with_dealings(&db.dealings_db)
            .with_proposal_db(&db.proposal_db)
            .with_verification_share(&db.verification_share_db)
            .with_threshold(&db.threshold_db)
            .with_initial_dealers_db(&db.initial_dealers_db),
        );
        let keypair = DkgKeyPair::new(dkg::params(), OsRng);
        let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        let state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            keypair,
            *identity_keypair.public_key(),
            KeyPair::new(),
        );
        let new_dkg_client2 = DkgClient::new(
            DummyClient::new(
                AccountId::from_str("n1sqkxzh7nl6kgndr4ew9795t2nkwmd8tpql67q7").unwrap(),
            )
            .with_dealer_details(&db.dealer_details_db)
            .with_dealings(&db.dealings_db)
            .with_proposal_db(&db.proposal_db)
            .with_verification_share(&db.verification_share_db)
            .with_threshold(&db.threshold_db)
            .with_initial_dealers_db(&db.initial_dealers_db),
        );
        let keypair = DkgKeyPair::new(dkg::params(), OsRng);
        let identity_keypair = identity::KeyPair::new(&mut thread_rng());
        let state2 = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            keypair,
            *identity_keypair.public_key(),
            KeyPair::new(),
        );

        for (_, active) in db.dealer_details_db.write().unwrap().values_mut() {
            *active = false;
        }

        *db.dealings_db.write().unwrap() = Default::default();
        *db.verification_share_db.write().unwrap() = Default::default();
        clients_and_states.pop().unwrap();
        let controller2 = clients_and_states.pop().unwrap();
        clients_and_states.push(DkgController::test_mock(new_dkg_client, state));
        clients_and_states.push(DkgController::test_mock(new_dkg_client2, state2));

        // DKG in reset mode
        for controller in clients_and_states.iter_mut() {
            controller.public_key_submission(0, false).await.unwrap();
        }
        for controller in clients_and_states.iter_mut() {
            dealing_exchange(&controller.dkg_client, &mut controller.state, OsRng, false)
                .await
                .unwrap();
        }
        for controller in clients_and_states.iter_mut() {
            let random_file: usize = OsRng.gen();
            let keypath = temp_dir().join(format!("coconut{}.pem", random_file));
            verification_key_submission(
                &controller.dkg_client,
                &mut controller.state,
                0,
                &keypath,
                false,
            )
            .await
            .unwrap();
            std::fs::remove_file(keypath).unwrap();
        }
        for controller in clients_and_states.iter_mut() {
            verification_key_validation(&controller.dkg_client, &mut controller.state, false)
                .await
                .unwrap();
        }
        for controller in clients_and_states.iter_mut() {
            verification_key_finalization(&controller.dkg_client, &mut controller.state, false)
                .await
                .unwrap();
        }
        assert!(db
            .proposal_db
            .read()
            .unwrap()
            .values()
            .all(|proposal| { proposal.status == Status::Executed }));
        for controller in clients_and_states.iter_mut() {
            controller.state.set_was_in_progress();
        }

        // DKG in reshare mode
        let mut vks = vec![];
        let mut indices = vec![];
        for controller in clients_and_states.iter() {
            let vk = controller
                .state
                .coconut_keypair()
                .await
                .as_ref()
                .unwrap()
                .keys
                .verification_key()
                .clone();
            let index = controller.state.node_index().unwrap();
            vks.push(vk);
            indices.push(index);
        }
        let initial_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();

        for (_, active) in db.dealer_details_db.write().unwrap().values_mut() {
            *active = false;
        }
        *db.dealings_db.write().unwrap() = Default::default();
        *db.verification_share_db.write().unwrap() = Default::default();
        let mut initial_dealers = vec![];
        for controller in clients_and_states.iter() {
            let client_address =
                Addr::unchecked(controller.dkg_client.get_address().await.as_ref());
            initial_dealers.push(client_address);
        }
        *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
            initial_dealers,
            initial_height: 1,
        });
        *clients_and_states.last_mut().unwrap() = controller2;

        for controller in clients_and_states.iter_mut() {
            controller.public_key_submission(0, true).await.unwrap();
        }

        for controller in clients_and_states.iter_mut() {
            dealing_exchange(&controller.dkg_client, &mut controller.state, OsRng, true)
                .await
                .unwrap();
        }

        for controller in clients_and_states.iter_mut() {
            let random_file: usize = OsRng.gen();
            let keypath = temp_dir().join(format!("coconut{}.pem", random_file));
            verification_key_submission(
                &controller.dkg_client,
                &mut controller.state,
                0,
                &keypath,
                true,
            )
            .await
            .unwrap();
            std::fs::remove_file(keypath).unwrap();
        }

        for controller in clients_and_states.iter_mut() {
            verification_key_validation(&controller.dkg_client, &mut controller.state, true)
                .await
                .unwrap();
        }
        for controller in clients_and_states.iter_mut() {
            verification_key_finalization(&controller.dkg_client, &mut controller.state, true)
                .await
                .unwrap();
        }
        // assert!(db
        //     .proposal_db
        //     .read()
        //     .unwrap()
        //     .values()
        //     .all(|proposal| { proposal.status == Status::Executed }));

        let mut vks = vec![];
        let mut indices = vec![];
        for controller in clients_and_states.iter() {
            let vk = controller
                .state
                .coconut_keypair()
                .await
                .as_ref()
                .unwrap()
                .keys
                .verification_key()
                .clone();
            let index = controller.state.node_index().unwrap();
            vks.push(vk);
            indices.push(index);
        }
        let reshared_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();
        assert_eq!(initial_master_vk, reshared_master_vk);
    }
}
