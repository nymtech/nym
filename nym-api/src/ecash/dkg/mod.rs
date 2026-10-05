// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::OnceLock;

pub(crate) fn params() -> &'static nym_dkg::bte::Params {
    static PARAMS: OnceLock<nym_dkg::bte::Params> = OnceLock::new();
    PARAMS.get_or_init(nym_dkg::bte::setup)
}

pub(crate) mod client;
pub(crate) mod controller;
pub(crate) mod dealing;
mod helpers;
pub(crate) mod key_derivation;
pub(crate) mod key_finalization;
pub(crate) mod key_validation;
pub(crate) mod public_key;
pub(crate) mod state;

#[cfg(test)]
mod tests {
    use crate::ecash::tests::contract_chain::SharedContractChain;
    use crate::ecash::tests::contract_harness;
    use nym_compact_ecash::aggregate_verification_keys;

    /// A full ceremony followed by a resharing, driven through the real coconut-dkg
    /// contract (with the real cw3 multisig and cw4 group) under `cw_multi_test`: state
    /// transitions go through `AdvanceEpochState` after passing real deadlines, the
    /// threshold is the contract's own computation, and share-verification proposals
    /// flow through the actual multisig.
    #[tokio::test]
    #[ignore] // expensive test
    async fn reshare_preserves_master_key() -> anyhow::Result<()> {
        let validators = 4;
        let chain = SharedContractChain::new(validators);
        let mut controllers = contract_harness::initialise_controllers(&chain);

        contract_harness::initiate_dkg(&chain);
        let epoch = chain.epoch().epoch_id;

        // EPOCH 0 DKG
        contract_harness::run_full_ceremony(&mut controllers, false).await;

        // the contract froze the threshold at ceil(2n/3) on entering dealing exchange
        assert_eq!(chain.epoch_threshold(epoch), Some(3));

        // get the master key
        let mut vks = vec![];
        let mut indices = vec![];
        for controller in controllers.iter() {
            vks.push(controller.unchecked_coconut_vk().await);
            indices.push(controller.state.assigned_index(epoch)?);
        }
        let initial_first_key = vks[0].clone();
        let initial_master_vk = aggregate_verification_keys(&vks, Some(&indices))?;

        // a fifth signer joins the group for the resharing epoch
        let joiner = chain.make_address("group-member-joiner".to_string());
        chain.add_group_member(joiner.clone());
        controllers.push(contract_harness::initialise_controller(
            &chain,
            joiner,
            validators as u8,
        ));

        contract_harness::trigger_resharing(&chain);
        let next_epoch = chain.epoch().epoch_id;

        // sanity check
        assert_eq!(next_epoch, epoch + 1);

        // EPOCH 1 DKG (resharing)
        contract_harness::run_full_ceremony(&mut controllers, true).await;

        let mut vks = vec![];
        let mut indices = vec![];
        for controller in controllers.iter() {
            vks.push(controller.unchecked_coconut_vk().await);
            indices.push(controller.state.assigned_index(next_epoch)?);
        }

        let updated_first_key = vks[0].clone();
        let reshared_master_vk = aggregate_verification_keys(&vks, Some(&indices))?;

        // individual keys changed
        assert_ne!(initial_first_key, updated_first_key);

        // but master didn't
        assert_eq!(initial_master_vk, reshared_master_vk);

        Ok(())
    }
}
