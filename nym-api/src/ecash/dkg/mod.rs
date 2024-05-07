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
    use crate::ecash::tests::helpers::{
        derive_keypairs, exchange_dealings, finalize, init_chain, initialise_controller,
        initialise_dkg, submit_public_keys, validate_keys,
    };
    use nym_compact_ecash::aggregate_verification_keys;

    #[tokio::test]
    #[ignore] // expensive test
    async fn reshare_preserves_master_key() -> anyhow::Result<()> {
        let validators = 4;
        let chain = init_chain();

        let mut controllers = vec![];
        for i in 0..validators {
            controllers.push(initialise_controller(chain.clone(), i).await)
        }

        let chain = controllers[0].chain_state.clone();
        let epoch = chain.lock().unwrap().dkg_contract.epoch.epoch_id;

        // EPOCH 0 DKG
        initialise_dkg(&mut controllers, false).await;
        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;
        derive_keypairs(&mut controllers, false).await;
        validate_keys(&mut controllers, false).await;
        finalize(&mut controllers).await;

        // get the master key
        let mut vks = vec![];
        let mut indices = vec![];
        for controller in controllers.iter() {
            let vk = controller.unchecked_coconut_vk().await;
            let index = controller.state.assigned_index(epoch)?;
            vks.push(vk);
            indices.push(index);
        }
        let initial_first_key = vks[0].clone();
        let initial_master_vk = aggregate_verification_keys(&vks, Some(&indices))?;

        let new_controller = initialise_controller(chain.clone(), validators).await;
        controllers.push(new_controller);

        chain.lock().unwrap().advance_epoch_in_reshare_mode();

        let next_epoch = epoch + 1;
        // sanity check
        assert_eq!(
            next_epoch,
            chain.lock().unwrap().dkg_contract.epoch.epoch_id
        );

        // EPOCH 1 DKG (resharing)
        submit_public_keys(&mut controllers, true).await;
        exchange_dealings(&mut controllers, true).await;
        derive_keypairs(&mut controllers, true).await;
        validate_keys(&mut controllers, true).await;
        finalize(&mut controllers).await;

        let mut vks = vec![];
        let mut indices = vec![];
        for controller in controllers.iter() {
            let vk = controller.unchecked_coconut_vk().await;
            let index = controller.state.assigned_index(next_epoch)?;
            vks.push(vk);
            indices.push(index);
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
