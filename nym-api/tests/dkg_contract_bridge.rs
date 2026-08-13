// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Spike: proves nym-api tests can drive the real coconut-dkg contract under
//! cw_multi_test, and that contract types unify with the `nym-coconut-dkg-common`
//! types nym-api itself depends on.

#![allow(clippy::unwrap_used)]

use nym_coconut_dkg::testable_dkg_contract::{
    init_contract_tester_with_group_members, DkgContractTesterExt,
};
use nym_coconut_dkg_common::types::EpochState;

#[test]
fn dkg_contract_runs_under_cw_multi_test() {
    let mut contract = init_contract_tester_with_group_members(4);

    // the type below is nym-api's own `nym_coconut_dkg_common::types::Epoch`;
    // this only compiles if both sides resolved to a single crate instance
    let epoch = contract.epoch();
    assert_eq!(epoch.epoch_id, 0);
    assert_eq!(epoch.state, EpochState::WaitingInitialisation);

    contract.run_initial_dummy_dkg();
    assert_eq!(contract.epoch().state, EpochState::InProgress);

    contract.run_resharing_dkg();
    assert_eq!(contract.epoch().epoch_id, 1);
}
