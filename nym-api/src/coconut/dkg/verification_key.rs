// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use crate::coconut::helpers::accepted_vote_err;
use cosmwasm_std::Addr;
use cw3::{ProposalResponse, Status};
use nym_coconut_dkg_common::event_attributes::DKG_PROPOSAL_ID;
use nym_coconut_dkg_common::types::{NodeIndex, TOTAL_DEALINGS};
use nym_coconut_dkg_common::verification_key::owner_from_cosmos_msgs;
use nym_coconut_interface::KeyPair as CoconutKeyPair;
use nym_credentials::coconut::bandwidth::{PRIVATE_ATTRIBUTES, PUBLIC_ATTRIBUTES};
use nym_dkg::bte::{decrypt_share, setup};
use nym_dkg::error::DkgError;
use nym_dkg::{combine_shares, try_recover_verification_keys, Dealing, Threshold};
use nym_pemstore::KeyPairPath;
use nymcoconut::tests::helpers::transpose_matrix;
use nymcoconut::{check_vk_pairing, Base58, KeyPair, Parameters, SecretKey, VerificationKey};
use std::collections::BTreeMap;
use validator_client::nyxd::cosmwasm_client::logs::find_attribute;

// Filter the dealers based on what dealing they posted (or not) in the contract
async fn deterministic_filter_dealers(
    dkg_client: &DkgClient,
    state: &mut State,
    threshold: Threshold,
    resharing: bool,
) -> Result<Vec<BTreeMap<NodeIndex, (Addr, Dealing)>>, CoconutError> {
    let mut dealings_maps = vec![];
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

    let params = setup();

    for idx in 0..TOTAL_DEALINGS {
        let dealings = dkg_client.get_dealings(idx).await?;
        let dealings_map =
            BTreeMap::from_iter(dealings.into_iter().filter_map(|contract_dealing| {
                match Dealing::try_from(&contract_dealing.dealing) {
                    Ok(dealing) => {
                        if dealing
                            .verify(&params, threshold, &initial_receivers, None)
                            .is_err()
                        {
                            state.mark_bad_dealer(
                                &contract_dealing.dealer,
                                ComplaintReason::DealingVerificationError,
                            );
                            None
                        } else if let Some(idx) =
                            initial_dealers_by_addr.get(&contract_dealing.dealer)
                        {
                            Some((*idx, (contract_dealing.dealer, dealing)))
                        } else {
                            None
                        }
                    }
                    Err(_) => {
                        state.mark_bad_dealer(
                            &contract_dealing.dealer,
                            ComplaintReason::MalformedDealing,
                        );
                        None
                    }
                }
            }));
        dealings_maps.push(dealings_map);
    }

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
        let recovered = try_recover_verification_keys(
            &filtered_dealings,
            threshold,
            &filtered_receivers_by_idx,
        )?;
        recovered_vks.push(recovered);

        let shares = filtered_dealings
            .iter()
            .map(|dealing| decrypt_share(dk, node_index_value, &dealing.ciphertexts, None))
            .collect::<Result<_, _>>()?;
        let scalar = combine_shares(shares, &filtered_dealers)?;
        scalars.push(scalar);
    }
    state.set_recovered_vks(recovered_vks);

    let params = Parameters::new(PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES)?;
    let x = scalars.pop().ok_or(CoconutError::DkgError(
        DkgError::NotEnoughDealingsAvailable {
            available: 0,
            required: 1,
        },
    ))?;
    let sk = SecretKey::create_from_raw(x, scalars);
    let vk = sk.verification_key(&params);

    Ok(CoconutKeyPair::from_keys(sk, vk))
}

pub(crate) async fn verification_key_submission(
    dkg_client: &DkgClient,
    state: &mut State,
    keypair_path: &KeyPairPath,
    resharing: bool,
) -> Result<(), CoconutError> {
    if state.coconut_keypair_is_some().await {
        return Ok(());
    }

    let threshold = state.threshold()?;
    let dealings_maps =
        deterministic_filter_dealers(dkg_client, state, threshold, resharing).await?;
    let coconut_keypair = derive_partial_keypair(state, threshold, dealings_maps)?;
    let vk_share = coconut_keypair.verification_key().to_bs58();
    nym_pemstore::store_keypair(&coconut_keypair, keypair_path)?;
    let res = dkg_client
        .submit_verification_key_share(vk_share, resharing)
        .await?;
    let proposal_id = find_attribute(&res.logs, "wasm", DKG_PROPOSAL_ID)
        .ok_or(CoconutError::ProposalIdError {
            reason: String::from("proposal id not found"),
        })?
        .value
        .parse::<u64>()
        .map_err(|_| CoconutError::ProposalIdError {
            reason: String::from("proposal id could not be parsed to u64"),
        })?;
    state.set_proposal_id(proposal_id);
    state.set_coconut_keypair(Some(coconut_keypair)).await;
    info!("DKG: Submitted own verification key");

    Ok(())
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
    let params = Parameters::new(PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES)?;
    for contract_share in vk_shares {
        if let Some(proposal_id) = proposal_ids.get(&contract_share.owner).copied() {
            match VerificationKey::try_from_bs58(contract_share.share) {
                Ok(vk) => {
                    if let Some(idx) = filtered_receivers_by_idx
                        .iter()
                        .position(|node_index| contract_share.node_index == *node_index)
                    {
                        let ret = if !check_vk_pairing(&params, &recovered_partials[idx], &vk) {
                            dkg_client
                                .vote_verification_key_share(proposal_id, false)
                                .await
                        } else {
                            dkg_client
                                .vote_verification_key_share(proposal_id, true)
                                .await
                        };
                        accepted_vote_err(ret)?;
                    }
                }
                Err(_) => {
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
    use crate::coconut::dkg::dealing::dealing_exchange;
    use crate::coconut::dkg::public_key::public_key_submission;
    use crate::coconut::dkg::state::PersistentState;
    use crate::coconut::tests::DummyClient;
    use crate::coconut::KeyPair;
    use nym_coconut_dkg_common::dealer::DealerDetails;
    use nym_coconut_dkg_common::types::InitialReplacementData;
    use nym_coconut_dkg_common::verification_key::ContractVKShare;
    use nym_contracts_common::dealings::ContractSafeBytes;
    use nym_dkg::bte::keys::KeyPair as DkgKeyPair;
    use nymcoconut::aggregate_verification_keys;
    use rand::rngs::OsRng;
    use rand::Rng;
    use std::collections::HashMap;
    use std::env::temp_dir;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};
    use url::Url;
    use validator_client::nyxd::AccountId;

    struct MockContractDb {
        dealer_details_db: Arc<RwLock<HashMap<String, DealerDetails>>>,
        dealings_db: Arc<RwLock<HashMap<String, Vec<ContractSafeBytes>>>>,
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

    const TEST_VALIDATORS_ADDRESS: [&str; 3] = [
        "n1aq9kakfgwqcufr23lsv644apavcntrsqsk4yus",
        "n1s9l3xr4g0rglvk4yctktmck3h4eq0gp6z2e20v",
        "n19kl4py32vsk297dm93ezem992cdyzdy4zuc2x6",
    ];

    async fn prepare_clients_and_states(db: &MockContractDb) -> Vec<(DkgClient, State)> {
        let params = setup();
        let mut clients_and_states = vec![];

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
            let keypair = DkgKeyPair::new(&params, OsRng);
            let state = State::new(
                PathBuf::default(),
                PersistentState::default(),
                Url::parse("localhost:8000").unwrap(),
                keypair,
                KeyPair::new(),
            );
            clients_and_states.push((dkg_client, state));
        }
        for (dkg_client, state) in clients_and_states.iter_mut() {
            public_key_submission(dkg_client, state, false)
                .await
                .unwrap();
        }
        clients_and_states
    }

    async fn prepare_clients_and_states_with_dealing(
        db: &MockContractDb,
    ) -> Vec<(DkgClient, State)> {
        let mut clients_and_states = prepare_clients_and_states(db).await;
        for (dkg_client, state) in clients_and_states.iter_mut() {
            dealing_exchange(dkg_client, state, OsRng, false)
                .await
                .unwrap();
        }
        clients_and_states
    }

    async fn prepare_clients_and_states_with_submission(
        db: &MockContractDb,
    ) -> Vec<(DkgClient, State)> {
        let mut clients_and_states = prepare_clients_and_states_with_dealing(db).await;
        for (dkg_client, state) in clients_and_states.iter_mut() {
            let random_file: usize = OsRng.gen();
            let private_key_path = temp_dir().join(format!("private{}.pem", random_file));
            let public_key_path = temp_dir().join(format!("public{}.pem", random_file));
            let keypair_path = KeyPairPath::new(private_key_path.clone(), public_key_path.clone());
            verification_key_submission(dkg_client, state, &keypair_path, false)
                .await
                .unwrap();
            std::fs::remove_file(private_key_path).unwrap();
            std::fs::remove_file(public_key_path).unwrap();
        }
        clients_and_states
    }

    async fn prepare_clients_and_states_with_validation(
        db: &MockContractDb,
    ) -> Vec<(DkgClient, State)> {
        let mut clients_and_states = prepare_clients_and_states_with_submission(db).await;
        for (dkg_client, state) in clients_and_states.iter_mut() {
            verification_key_validation(dkg_client, state, false)
                .await
                .unwrap();
        }
        clients_and_states
    }

    async fn prepare_clients_and_states_with_finalization(
        db: &MockContractDb,
    ) -> Vec<(DkgClient, State)> {
        let mut clients_and_states = prepare_clients_and_states_with_validation(db).await;
        for (dkg_client, state) in clients_and_states.iter_mut() {
            verification_key_finalization(dkg_client, state, false)
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
        for (dkg_client, state) in clients_and_states.iter_mut() {
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            assert_eq!(filtered.len(), TOTAL_DEALINGS);
            for mapping in filtered.iter() {
                assert_eq!(mapping.len(), 3);
            }
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn check_dealers_filter_one_bad_dealing() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_dealing(&db).await;

        // corrupt just one dealing
        db.dealings_db
            .write()
            .unwrap()
            .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
            .and_modify(|dealings| {
                let mut last = dealings.pop().unwrap();
                last.0.pop();
                dealings.push(last);
            });

        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            assert_eq!(filtered.len(), TOTAL_DEALINGS);
            let corrupted_status = state
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

        // add all but the first dealing
        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            dealing_exchange(dkg_client, state, OsRng, true)
                .await
                .unwrap();
        }

        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
                initial_dealers: vec![Addr::unchecked(TEST_VALIDATORS_ADDRESS[0])],
                initial_height: None,
            });
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, true)
                .await
                .unwrap();
            assert_eq!(filtered.len(), TOTAL_DEALINGS);
            let corrupted_status = state
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

        // add all but the first dealing
        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            dealing_exchange(dkg_client, state, OsRng, true)
                .await
                .unwrap();
        }

        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
                initial_dealers: vec![],
                initial_height: None,
            });
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, true)
                .await
                .unwrap();
            assert_eq!(filtered.len(), TOTAL_DEALINGS);
            assert!(state
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

        // corrupt all dealings of one address
        db.dealings_db
            .write()
            .unwrap()
            .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
            .and_modify(|dealings| {
                dealings.iter_mut().for_each(|dealing| {
                    dealing.0.pop();
                });
            });

        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            assert_eq!(filtered.len(), TOTAL_DEALINGS);
            for mapping in filtered.iter() {
                assert_eq!(mapping.len(), 2);
            }
            let corrupted_status = state
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

        // corrupt just one dealing
        db.dealings_db
            .write()
            .unwrap()
            .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
            .and_modify(|dealings| {
                let mut last = dealings.pop().unwrap();
                last.0.pop();
                dealings.push(last);
            });

        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            // second filter will leave behind the bad dealer and surface why it was left out
            // in the first place
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            assert_eq!(filtered.len(), TOTAL_DEALINGS);
            let corrupted_status = state
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

        // corrupt just one dealing
        db.dealings_db
            .write()
            .unwrap()
            .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
            .and_modify(|dealings| {
                let mut last = dealings.pop().unwrap();
                let value = last.0.pop().unwrap();
                if value == 42 {
                    last.0.push(43);
                } else {
                    last.0.push(42);
                }
                dealings.push(last);
            });

        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            // second filter will leave behind the bad dealer and surface why it was left out
            // in the first place
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            assert_eq!(filtered.len(), TOTAL_DEALINGS);
            let corrupted_status = state
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
        for (dkg_client, state) in clients_and_states.iter_mut() {
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            assert!(derive_partial_keypair(state, 2, filtered).is_ok());
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
            .entry(TEST_VALIDATORS_ADDRESS[0].to_string())
            .and_modify(|dealings| {
                let mut last = dealings.pop().unwrap();
                last.0.pop();
                dealings.push(last);
            });

        for (dkg_client, state) in clients_and_states.iter_mut().skip(1) {
            let filtered = deterministic_filter_dealers(dkg_client, state, 2, false)
                .await
                .unwrap();
            assert!(derive_partial_keypair(state, 2, filtered).is_ok());
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn submit_verification_key() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_submission(&db).await;

        for (_, state) in clients_and_states.iter_mut() {
            assert!(db
                .proposal_db
                .read()
                .unwrap()
                .contains_key(&state.proposal_id_value().unwrap()));
            assert!(state.coconut_keypair_is_some().await);
        }
    }

    #[tokio::test]
    #[ignore] // expensive test
    async fn validate_verification_key() {
        let db = MockContractDb::new();
        let mut clients_and_states = prepare_clients_and_states_with_validation(&db).await;
        for (_, state) in clients_and_states.iter_mut() {
            let proposal = db
                .proposal_db
                .read()
                .unwrap()
                .get(&state.proposal_id_value().unwrap())
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

        for (dkg_client, state) in clients_and_states.iter_mut() {
            verification_key_validation(dkg_client, state, false)
                .await
                .unwrap();
        }

        for (idx, (_, state)) in clients_and_states.iter().enumerate() {
            let proposal = db
                .proposal_db
                .read()
                .unwrap()
                .get(&state.proposal_id_value().unwrap())
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

        for (dkg_client, state) in clients_and_states.iter_mut() {
            verification_key_validation(dkg_client, state, false)
                .await
                .unwrap();
        }

        for (idx, (_, state)) in clients_and_states.iter().enumerate() {
            let proposal = db
                .proposal_db
                .read()
                .unwrap()
                .get(&state.proposal_id_value().unwrap())
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

        for (_, state) in clients_and_states.iter() {
            let proposal = db
                .proposal_db
                .read()
                .unwrap()
                .get(&state.proposal_id_value().unwrap())
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
        let params = Parameters::new(4).unwrap();

        let mut vks = vec![];
        let mut indices = vec![];
        for (_, state) in clients_and_states.iter() {
            let vk = state
                .coconut_secret_key()
                .await
                .unwrap()
                .verification_key(&params);
            let index = state.node_index().unwrap();
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
        let keypair = DkgKeyPair::new(&setup(), OsRng);
        let state = State::new(
            PathBuf::default(),
            PersistentState::default(),
            Url::parse("localhost:8000").unwrap(),
            keypair,
            KeyPair::new(),
        );

        let removed_dealer = clients_and_states.first().unwrap().0.get_address().await;
        db.dealer_details_db
            .write()
            .unwrap()
            .remove(removed_dealer.as_ref());
        *db.dealings_db.write().unwrap() = Default::default();
        *db.verification_share_db.write().unwrap() = Default::default();
        *db.initial_dealers_db.write().unwrap() = Some(InitialReplacementData {
            initial_dealers: vec![
                Addr::unchecked(clients_and_states[1].0.get_address().await.as_ref()),
                Addr::unchecked(clients_and_states[2].0.get_address().await.as_ref()),
            ],
            initial_height: None,
        });
        *clients_and_states.first_mut().unwrap() = (new_dkg_client, state);
        clients_and_states[1].1.set_was_in_progress();
        clients_and_states[2].1.set_was_in_progress();

        for (dkg_client, state) in clients_and_states.iter_mut() {
            public_key_submission(dkg_client, state, true)
                .await
                .unwrap();
        }

        for (dkg_client, state) in clients_and_states.iter_mut() {
            dealing_exchange(dkg_client, state, OsRng, true)
                .await
                .unwrap();
        }

        for (dkg_client, state) in clients_and_states.iter_mut() {
            let random_file: usize = OsRng.gen();
            let private_key_path = temp_dir().join(format!("private{}.pem", random_file));
            let public_key_path = temp_dir().join(format!("public{}.pem", random_file));
            let keypair_path = KeyPairPath::new(private_key_path.clone(), public_key_path.clone());
            verification_key_submission(dkg_client, state, &keypair_path, true)
                .await
                .unwrap();
            std::fs::remove_file(private_key_path).unwrap();
            std::fs::remove_file(public_key_path).unwrap();
        }
        let mut vks = vec![];
        let mut indices = vec![];
        for (_, state) in clients_and_states.iter() {
            let vk = state
                .coconut_secret_key()
                .await
                .unwrap()
                .verification_key(&params);
            let index = state.node_index().unwrap();
            vks.push(vk);
            indices.push(index);
        }
        let reshared_master_vk = aggregate_verification_keys(&vks, Some(&indices)).unwrap();
        assert_eq!(initial_master_vk, reshared_master_vk);
    }
}
