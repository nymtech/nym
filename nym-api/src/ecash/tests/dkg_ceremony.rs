// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Tests about the ceremony as the *contract* runs it, rather than about any one api's
//! behaviour within it. Everything here goes through real transactions against the real
//! coconut-dkg contract, so a claim about what the chain does can be checked rather than
//! assumed.

use crate::ecash::tests::contract_chain::SharedContractChain;
use crate::ecash::tests::contract_harness::{cheap, initiate_dkg};
use nym_coconut_dkg_common::types::EpochState;

/// A ceremony with no dealers waits rather than starting, and the waiting is not passive
/// bookkeeping: dealers must still be able to register long after the original submission
/// deadline has gone by, or the hold would just be a different way of getting stuck.
#[test]
fn registration_stays_open_while_a_ceremony_waits_for_dealers() {
    let chain = SharedContractChain::new(4);
    initiate_dkg(&chain);
    let epoch_id = chain.epoch().epoch_id;

    // every api is still down, so the submission window rolls instead of the ceremony starting
    for _ in 0..3 {
        cheap::advance(&chain);

        let epoch = chain.epoch();
        assert_eq!(
            epoch.state,
            EpochState::PublicKeySubmission { resharing: false }
        );
        assert_eq!(epoch.epoch_id, epoch_id);
    }

    // they come back well past the deadline the epoch started with, and register as normal
    cheap::register_dealers(&chain, false);
    cheap::advance(&chain);

    assert_eq!(
        chain.epoch().state,
        EpochState::DealingExchange { resharing: false }
    );
    assert_eq!(chain.epoch_threshold(epoch_id), Some(3));
}

/// A ceremony that ends with too few verified shares resets to a fresh epoch rather than
/// concluding. That is deliberate - no credentials could be issued anyway - but it is only
/// a sound policy if the retry can actually succeed, otherwise the reset is an endless loop
/// that burns an epoch id per cycle.
///
/// So: the same group that failed must be able to re-register and conclude the retry.
#[test]
fn a_reset_ceremony_concludes_once_participation_recovers() {
    let chain = SharedContractChain::new(4);
    initiate_dkg(&chain);
    let first_epoch = chain.epoch().epoch_id;

    // four dealers register, so the contract wants three verified shares
    cheap::register_dealers(&chain, false);
    cheap::advance(&chain);
    assert_eq!(chain.epoch_threshold(first_epoch), Some(3));

    cheap::submit_dealings(&chain, false);
    cheap::advance(&chain);
    cheap::submit_vk_shares(&chain, false);
    cheap::advance(&chain);
    cheap::advance(&chain);

    // but only two of them make it through verification
    cheap::verify_first_vk_shares(&chain, false, 2);
    cheap::advance(&chain);

    let epoch = chain.epoch();
    assert_eq!(
        epoch.state,
        EpochState::PublicKeySubmission { resharing: false }
    );
    assert_eq!(epoch.epoch_id, first_epoch + 1);

    // participation recovers, and the retry is not blocked by the failed attempt: the same
    // dealers register again for the new epoch and see it through
    cheap::run_ceremony(&chain, false);

    let epoch = chain.epoch();
    assert_eq!(epoch.state, EpochState::InProgress);
    assert_eq!(epoch.epoch_id, first_epoch + 1);
    for member in chain.group_member_addresses() {
        assert!(chain.vk_share_verified(epoch.epoch_id, &member));
    }
}
