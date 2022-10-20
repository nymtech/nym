// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::{DkgParticipant, State};
use crate::coconut::error::CoconutError;
use contracts_common::dealings::ContractSafeBytes;
use dkg::bte::setup;
use dkg::Dealing;
use rand::RngCore;
use std::collections::BTreeMap;

pub(crate) async fn dealing_exchange(
    dkg_client: &DkgClient,
    state: &mut State,
    rng: impl RngCore,
) -> Result<(), CoconutError> {
    if state.self_share().is_some() {
        return Ok(());
    }

    let dealers = dkg_client.get_current_dealers().await?;
    // note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
    let threshold = (2 * dealers.len() as u64 + 3 - 1) / 3;
    dealers.into_iter().for_each(|dealer| {
        let node_index = dealer.assigned_index;
        match DkgParticipant::try_from(dealer) {
            Ok(participant) => state.add_good_dealer(participant),
            Err(reason) => state.add_bad_dealer(node_index, reason),
        };
    });

    let dkg_receivers = state
        .current_dealers()
        .iter()
        .map(|(idx, participant)| (*idx, *participant.bte_public_key_with_proof.public_key()));

    let (dealing, self_share) = Dealing::create(
        rng,
        &setup(),
        state
            .node_index()
            .expect("Node index should be initialized"),
        threshold,
        &BTreeMap::from_iter(dkg_receivers),
        None,
    );
    state.set_self_share(self_share);

    let dealing_bytes = ContractSafeBytes::from(&dealing);
    dkg_client.submit_dealing(dealing_bytes).await
}
