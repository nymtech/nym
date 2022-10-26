// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::State;
use crate::coconut::error::CoconutError;
use contracts_common::dealings::ContractSafeBytes;
use dkg::bte::setup;
use dkg::Dealing;
use rand::RngCore;

pub(crate) async fn dealing_exchange(
    dkg_client: &DkgClient,
    state: &mut State,
    rng: impl RngCore,
) -> Result<(), CoconutError> {
    if state.self_share().is_some() {
        return Ok(());
    }

    if state.current_receivers().is_empty() {
        // First initialization with the dealers from contract
        let dealers = dkg_client.get_current_dealers().await?;
        state.set_dealers(dealers);
    }

    let receivers = state.current_receivers();
    // note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
    let threshold = (2 * receivers.len() as u64 + 3 - 1) / 3;

    let (dealing, self_share) = Dealing::create(
        rng,
        &setup(),
        state
            .node_index()
            .expect("Node index should be initialized"),
        threshold,
        &receivers,
        None,
    );
    state.set_self_share(self_share);

    let dealing_bytes = ContractSafeBytes::from(&dealing);
    dkg_client.submit_dealing(dealing_bytes).await
}
