// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use coconut_dkg_common::types::{NodeIndex, TOTAL_DEALINGS};
use coconut_interface::KeyPair as CoconutKeyPair;
use cosmwasm_std::Addr;
use credentials::coconut::bandwidth::{PRIVATE_ATTRIBUTES, PUBLIC_ATTRIBUTES};
use dkg::bte::{decrypt_share, setup};
use dkg::{combine_shares, Dealing};
use nymcoconut::{KeyPair, Parameters, SecretKey};
use std::collections::BTreeMap;

// Filter the dealers based on what dealing they posted (or not) in the contract
async fn deterministic_filter_dealers(
    dkg_client: &DkgClient,
    state: &mut State,
) -> Result<Vec<BTreeMap<NodeIndex, (Addr, Dealing)>>, CoconutError> {
    let mut dealings_maps = vec![];
    let initial_dealers_by_addr = state.current_dealers_by_addr();
    let initial_receivers = state.current_dealers_by_idx();
    let threshold = state.threshold()?;
    let params = setup();
    let retries = 3;

    for idx in 0..TOTAL_DEALINGS {
        let mut try_no = 0;
        let dealings = loop {
            // this is a really ugly way to get the dealings, but for some reason the first query
            // always fails with a RPC error.
            try_no += 1;
            if let Ok(dealings) = dkg_client.get_dealings(idx).await {
                break dealings;
            } else if try_no == retries {
                return Err(CoconutError::UnrecoverableState {
                    reason: String::from("Could not get dealings"),
                });
            }
        };
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
    state: &State,
    dealings_maps: Vec<BTreeMap<NodeIndex, (Addr, Dealing)>>,
) -> Result<KeyPair, CoconutError> {
    let filtered_receivers_by_idx = state.current_dealers_by_idx();
    let filtered_dealers_by_addr = state.current_dealers_by_addr();
    let dk = state.dkg_keypair().private_key();
    let node_index_value = state.receiver_index_value()?;
    let mut scalars = vec![];
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

    let params = Parameters::new(PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES)?;
    let x = scalars.pop().unwrap();
    let sk = SecretKey::create_from_raw(x, scalars);
    let vk = sk.verification_key(&params);

    Ok(CoconutKeyPair::from_keys(sk, vk))
}

pub(crate) async fn verification_key_submission(
    dkg_client: &DkgClient,
    state: &mut State,
) -> Result<(), CoconutError> {
    if state.coconut_keypair_is_some().await {
        return Ok(());
    }

    let dealings_maps = deterministic_filter_dealers(dkg_client, state).await?;
    let coconut_keypair = derive_partial_keypair(state, dealings_maps)?;
    state.set_coconut_keypair(coconut_keypair).await;

    Ok(())
}
