// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use coconut_dkg_common::types::TOTAL_DEALINGS;
use contracts_common::dealings::ContractSafeBytes;
use dkg::bte::setup;
use dkg::Dealing;
use rand::RngCore;

pub(crate) async fn dealing_exchange(
    dkg_client: &DkgClient,
    state: &mut State,
    rng: impl RngCore + Clone,
) -> Result<(), CoconutError> {
    if state.receiver_index().is_some() {
        return Ok(());
    }

    let dealers = dkg_client.get_current_dealers().await?;
    // note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
    let threshold = (2 * dealers.len() as u64 + 3 - 1) / 3;

    state.set_dealers(dealers);
    state.set_threshold(threshold);
    let receivers = state.current_dealers_by_idx();
    let params = setup();
    let dealer_index = state.node_index_value()?;
    let receiver_index = receivers
        .keys()
        .position(|node_index| *node_index == dealer_index);
    for _ in 0..TOTAL_DEALINGS {
        let (dealing, _) = Dealing::create(
            rng.clone(),
            &params,
            dealer_index,
            threshold,
            &receivers,
            None,
        );
        dkg_client
            .submit_dealing(ContractSafeBytes::from(&dealing))
            .await?;
    }

    info!("Finished submitting DKG dealing");
    state.set_receiver_index(receiver_index);

    Ok(())
}
