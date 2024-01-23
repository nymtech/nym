// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::dkg::client::DkgClient;
use crate::coconut::dkg::controller::DkgController;
use crate::coconut::dkg::state::{ConsistentState, State};
use crate::coconut::error::CoconutError;
use rand::{CryptoRng, RngCore};

impl<R: RngCore + CryptoRng> DkgController<R> {}

pub(crate) async fn verification_key_finalization(
    dkg_client: &DkgClient,
    state: &mut State,
    _resharing: bool,
) -> Result<(), CoconutError> {
    if state.executed_proposal() {
        log::debug!("Already executed the proposal, nothing to do");
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
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // expensive test
    async fn finalize_verification_key() {
        todo!()
        // let db = MockContractDb::new();
        // let clients_and_states = prepare_clients_and_states_with_finalization(&db).await;
        //
        // for controller in clients_and_states.iter() {
        //     let proposal = db
        //         .proposal_db
        //         .read()
        //         .unwrap()
        //         .get(&controller.state.proposal_id_value().unwrap())
        //         .unwrap()
        //         .clone();
        //     assert_eq!(proposal.status, Status::Executed);
        // }
    }
}
