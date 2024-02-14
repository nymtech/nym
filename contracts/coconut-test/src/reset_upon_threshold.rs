// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::test_wrapper::TestSetup;
use nym_coconut_dkg_common::types::EpochState;

#[test]
fn removing_enough_members_causes_dkg_reset() {
    let mut test = TestSetup::new();

    let member1 = test.add_mock_group_member(None);
    let member2 = test.add_mock_group_member(None);
    let member3 = test.add_mock_group_member(None);
    let member4 = test.add_mock_group_member(None);
    let member5 = test.add_mock_group_member(None);

    test.begin_dkg();
    test.full_dummy_dkg(
        vec![
            member1,
            member2,
            member3.clone(),
            member4.clone(),
            member5.clone(),
        ],
        false,
    );

    test.remove_group_member(&member3);
    test.remove_group_member(&member4);
    test.remove_group_member(&member5);

    test.skip_to_dkg_state_end();
    test.unchecked_advance_dkg_epoch();

    let epoch = test.epoch();
    assert_eq!(
        epoch.state,
        EpochState::PublicKeySubmission { resharing: false }
    );
}

#[test]
fn adding_enough_members_causes_dkg_reset() {
    let mut test = TestSetup::new();

    let member1 = test.add_mock_group_member(None);
    let member2 = test.add_mock_group_member(None);

    test.begin_dkg();
    test.full_dummy_dkg(vec![member1, member2], false);

    let _member3 = test.add_mock_group_member(None);
    let _member4 = test.add_mock_group_member(None);
    let _member5 = test.add_mock_group_member(None);

    test.skip_to_dkg_state_end();
    test.unchecked_advance_dkg_epoch();

    let epoch = test.epoch();
    assert_eq!(
        epoch.state,
        EpochState::PublicKeySubmission { resharing: false }
    );
}
