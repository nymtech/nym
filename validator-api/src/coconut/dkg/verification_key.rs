// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use coconut_dkg_common::event_attributes::DKG_PROPOSAL_ID;
use coconut_dkg_common::types::{NodeIndex, TOTAL_DEALINGS};
use coconut_dkg_common::verification_key::owner_from_cosmos_msgs;
use coconut_interface::KeyPair as CoconutKeyPair;
use cosmwasm_std::Addr;
use credentials::coconut::bandwidth::{PRIVATE_ATTRIBUTES, PUBLIC_ATTRIBUTES};
use cw3::{ProposalResponse, Status};
use dkg::bte::{decrypt_share, setup};
use dkg::{combine_shares, try_recover_verification_keys, Dealing, Threshold};
use nymcoconut::tests::helpers::transpose_matrix;
use nymcoconut::{check_vk_pairing, Base58, KeyPair, Parameters, SecretKey, VerificationKey};
use pemstore::KeyPairPath;
use std::collections::BTreeMap;
use validator_client::nymd::cosmwasm_client::logs::find_attribute;

// Filter the dealers based on what dealing they posted (or not) in the contract
async fn deterministic_filter_dealers(
    dkg_client: &DkgClient,
    state: &mut State,
    threshold: Threshold,
) -> Result<Vec<BTreeMap<NodeIndex, (Addr, Dealing)>>, CoconutError> {
    let mut dealings_maps = vec![];
    let initial_dealers_by_addr = state.current_dealers_by_addr();
    let initial_receivers = state.current_dealers_by_idx();
    let params = setup();

    for idx in 0..TOTAL_DEALINGS {
        let dealings = dkg_client.get_dealings(idx).await?;
        let dealings_map =
            BTreeMap::from_iter(dealings.into_iter().filter_map(|contract_dealing| {
                match Dealing::try_from(&contract_dealing.dealing) {
                    Ok(dealing) => {
                        if let Err(err) =
                            dealing.verify(&params, threshold, &initial_receivers, None)
                        {
                            state.mark_bad_dealer(
                                &contract_dealing.dealer,
                                ComplaintReason::DealingVerificationError(err),
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
                    Err(err) => {
                        state.mark_bad_dealer(
                            &contract_dealing.dealer,
                            ComplaintReason::MalformedDealing(err),
                        );
                        None
                    }
                }
            }));
        dealings_maps.push(dealings_map);
    }
    for (addr, _) in initial_dealers_by_addr.iter() {
        for dealings_map in dealings_maps.iter() {
            if !dealings_map.iter().any(|(_, (address, _))| address == addr) {
                state.mark_bad_dealer(addr, ComplaintReason::MissingDealing);
                break;
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
        let filtered_dealings: Vec<_> = dealings_map
            .into_iter()
            .filter_map(|(_, (addr, dealing))| {
                if filtered_dealers_by_addr.keys().any(|a| addr == *a) {
                    Some(dealing)
                } else {
                    None
                }
            })
            .collect();
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
        let scalar = combine_shares(
            shares,
            &filtered_receivers_by_idx
                .keys()
                .copied()
                .collect::<Vec<_>>(),
        )?;
        scalars.push(scalar);
    }
    state.set_recovered_vks(recovered_vks);

    let params = Parameters::new(PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES)?;
    let x = scalars.pop().unwrap();
    let sk = SecretKey::create_from_raw(x, scalars);
    let vk = sk.verification_key(&params);

    Ok(CoconutKeyPair::from_keys(sk, vk))
}

pub(crate) async fn verification_key_submission(
    dkg_client: &DkgClient,
    state: &mut State,
    keypair_path: &KeyPairPath,
) -> Result<(), CoconutError> {
    if state.coconut_keypair_is_some().await {
        return Ok(());
    }

    let threshold = state.threshold()?;
    let dealings_maps = deterministic_filter_dealers(dkg_client, state, threshold).await?;
    let coconut_keypair = derive_partial_keypair(state, threshold, dealings_maps)?;
    let vk_share = coconut_keypair.verification_key().to_bs58();
    pemstore::store_keypair(&coconut_keypair, keypair_path)?;
    let res = dkg_client.submit_verification_key_share(vk_share).await?;
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
    state.set_coconut_keypair(coconut_keypair).await;
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
) -> Result<(), CoconutError> {
    if state.voted_vks() {
        return Ok(());
    }

    let vk_shares = dkg_client.get_verification_key_shares().await?;
    let proposal_ids = BTreeMap::from_iter(
        dkg_client
            .list_proposals()
            .await?
            .iter()
            .filter_map(|prop| validate_proposal(prop)),
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
                        if !check_vk_pairing(&params, &recovered_partials[idx], &vk) {
                            dkg_client
                                .vote_verification_key_share(proposal_id, false)
                                .await?;
                        } else {
                            dkg_client
                                .vote_verification_key_share(proposal_id, true)
                                .await?;
                        }
                    }
                }
                Err(_) => {
                    dkg_client
                        .vote_verification_key_share(proposal_id, false)
                        .await?
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
