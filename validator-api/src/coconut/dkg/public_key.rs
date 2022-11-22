// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::state::State;
use crate::coconut::error::CoconutError;

pub(crate) async fn public_key_submission(
    dkg_client: &DkgClient,
    state: &mut State,
) -> Result<(), CoconutError> {
    if state.node_index().is_some() {
        return Ok(());
    }

    let bte_key = bs58::encode(&state.dkg_keypair().public_key().to_bytes()).into_string();
    let index = if let Some(details) = dkg_client
        .get_self_registered_dealer_details()
        .await?
        .details
    {
        details.assigned_index
    } else {
        dkg_client
            .register_dealer(bte_key, state.announce_address().to_string())
            .await?
    };
    state.set_node_index(index);
    info!("DKG: Using node index {}", index);

    Ok(())
}
