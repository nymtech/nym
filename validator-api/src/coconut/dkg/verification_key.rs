// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::complaints::ComplaintReason;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::dkg::utils::transpose_matrix;
use crate::coconut::error::CoconutError;
use coconut_dkg_common::dealer::ContractDealing;
use coconut_dkg_common::types::TOTAL_DEALINGS;
use contracts_common::dealings::ContractSafeBytes;
use cosmwasm_std::Addr;
use dkg::bte::{setup, PublicKey};
use dkg::error::DkgError;
use dkg::{try_recover_verification_keys, Dealing, NodeIndex, Threshold};
use std::collections::BTreeMap;

fn filter_out_dealings_in_batch(
    state: &mut State,
    dealings_bytes: &[ContractSafeBytes; TOTAL_DEALINGS],
    addr: &Addr,
    initial_receivers: &BTreeMap<NodeIndex, PublicKey>,
    threshold: Threshold,
) -> Option<[Dealing; TOTAL_DEALINGS]> {
    let mut dealings_batch = Vec::new();
    for dealing_bytes in dealings_bytes {
        match Dealing::try_from(dealing_bytes) {
            Ok(dealing) => {
                if let Err(err) = dealing.verify(&setup(), threshold, initial_receivers, None) {
                    state.mark_bad_dealer(addr, ComplaintReason::DealingVerificationError(err));
                    break;
                } else {
                    dealings_batch.push(dealing);
                }
            }
            Err(err) => {
                state.mark_bad_dealer(addr, ComplaintReason::MalformedDealing(err));
                break;
            }
        }
    }
    dealings_batch.try_into().ok()
}

fn filter_out_bad_dealers(
    state: &mut State,
    dealings: Vec<ContractDealing>,
) -> Result<[Vec<Dealing>; TOTAL_DEALINGS], CoconutError> {
    let dealings_map = BTreeMap::from_iter(
        dealings
            .into_iter()
            .map(|dealing| (dealing.dealer, dealing.dealings)),
    );
    let initial_receivers = state.current_receivers();
    let initial_dealers = state.current_dealers();
    let threshold = state.threshold()?;
    let mut dealings = Vec::new();
    for addr in initial_dealers {
        match dealings_map.get(&addr) {
            Some(dealings_bytes) => {
                if let Some(dealings_batch) = filter_out_dealings_in_batch(
                    state,
                    dealings_bytes,
                    &addr,
                    &initial_receivers,
                    threshold,
                ) {
                    dealings.push(dealings_batch);
                }
            }
            None => state.mark_bad_dealer(&addr, ComplaintReason::MissingDealing),
        }
    }
    Ok(transpose_matrix(dealings))
}

pub(crate) async fn verification_key_submission(
    dkg_client: &DkgClient,
    state: &mut State,
) -> Result<(), CoconutError> {
    let dealings = dkg_client.get_dealings().await?;
    let filtered_batched_dealings = filter_out_bad_dealers(state, dealings)?;

    let threshold = state.threshold()?;
    let receivers = state.current_receivers();
    let ret = filtered_batched_dealings
        .map(|filtered_dealings| {
            try_recover_verification_keys(&filtered_dealings, threshold, &receivers)
        })
        .into_iter()
        .collect::<Result<Vec<_>, DkgError>>()?;

    Ok(())
}
