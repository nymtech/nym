// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::{
    contract_dkg, contract_group, contract_multisig, mock_app, MigrateMsg, MEMBER1, OWNER,
};
use crate::spend_credential_creates_proposal::{
    TEST_COCONUT_BANDWIDTH_CONTRACT_ADDRESS, TEST_COCONUT_DKG_CONTRACT_ADDRESS, TEST_COIN_DENOM,
};
use coconut_dkg_common::msg::ExecuteMsg::{
    AdvanceEpochState, CommitVerificationKeyShare, RegisterDealer,
};
use coconut_dkg_common::msg::InstantiateMsg as DkgInstantiateMsg;
use coconut_dkg_common::msg::QueryMsg::GetVerificationKeys;
use coconut_dkg_common::verification_key::PagedVKSharesResponse;
use cosmwasm_std::{coins, Addr, Decimal};
use cw4::Member;
use cw4_group::msg::InstantiateMsg as GroupInstantiateMsg;
use cw_multi_test::Executor;
use cw_utils::{Duration, Threshold};
use multisig_contract_common::msg::ExecuteMsg::{Execute, Vote};
use multisig_contract_common::msg::InstantiateMsg as MultisigInstantiateMsg;

#[test]
fn dkg_proposal() {
    let init_funds = coins(10000000000, TEST_COIN_DENOM);
    let mut app = mock_app(&init_funds);
    let member1 = Member {
        addr: MEMBER1.to_string(),
        weight: 10,
    };

    let group_code_id = app.store_code(contract_group());
    let msg = GroupInstantiateMsg {
        admin: Some(OWNER.to_string()),
        members: vec![member1],
    };
    let group_contract_addr = app
        .instantiate_contract(
            group_code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "group",
            None,
        )
        .unwrap();

    let multisig_code_id = app.store_code(contract_multisig());
    let msg = MultisigInstantiateMsg {
        group_addr: group_contract_addr.to_string(),
        threshold: Threshold::AbsolutePercentage {
            percentage: Decimal::from_ratio(1u128, 1u128),
        },
        max_voting_period: Duration::Time(1000),
        executor: None,
        coconut_bandwidth_contract_address: TEST_COCONUT_BANDWIDTH_CONTRACT_ADDRESS.to_string(),
        coconut_dkg_contract_address: TEST_COCONUT_DKG_CONTRACT_ADDRESS.to_string(),
        proposal_deposit: None,
    };
    let multisig_contract_addr = app
        .instantiate_contract(
            multisig_code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "multisig",
            Some(OWNER.to_string()),
        )
        .unwrap();

    let coconut_dkg_code_id = app.store_code(contract_dkg());
    let msg = DkgInstantiateMsg {
        group_addr: group_contract_addr.to_string(),
        multisig_addr: multisig_contract_addr.to_string(),
        admin: Addr::unchecked(OWNER).to_string(),
        mix_denom: TEST_COIN_DENOM.to_string(),
    };
    let coconut_dkg_contract_addr = app
        .instantiate_contract(
            coconut_dkg_code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "coconut dkg",
            None,
        )
        .unwrap();

    let msg = MigrateMsg {
        coconut_bandwidth_address: coconut_dkg_contract_addr.to_string(),
        coconut_dkg_address: coconut_dkg_contract_addr.to_string(),
    };
    app.migrate_contract(
        Addr::unchecked(OWNER),
        multisig_contract_addr.clone(),
        &msg,
        multisig_code_id,
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(MEMBER1),
        coconut_dkg_contract_addr.clone(),
        &RegisterDealer {
            bte_key_with_proof: "bte_key_with_proof".to_string(),
            announce_address: "127.0.0.1:8000".to_string(),
        },
        &vec![],
    )
    .unwrap();

    for _ in 0..2 {
        app.execute_contract(
            Addr::unchecked(OWNER),
            coconut_dkg_contract_addr.clone(),
            &AdvanceEpochState {},
            &vec![],
        )
        .unwrap();
    }

    // Proposal needs to be later then the member became part of the group
    app.update_block(|block| block.height += 1);

    let msg = CommitVerificationKeyShare {
        share: "share".to_string(),
    };
    let res = app
        .execute_contract(
            Addr::unchecked(MEMBER1),
            coconut_dkg_contract_addr.clone(),
            &msg,
            &vec![],
        )
        .unwrap();

    let proposal_id = res
        .events
        .into_iter()
        .find(|e| &e.ty == "wasm")
        .unwrap()
        .attributes
        .into_iter()
        .find(|attr| &attr.key == "proposal_id")
        .unwrap()
        .value
        .parse::<u64>()
        .unwrap();

    let mut res: PagedVKSharesResponse = app
        .wrap()
        .query_wasm_smart(
            coconut_dkg_contract_addr.clone(),
            &GetVerificationKeys {
                limit: None,
                start_after: None,
            },
        )
        .unwrap();
    let share = res.shares.pop().unwrap();
    assert_eq!(share.share, "share".to_string());
    assert_eq!(share.announce_address, "127.0.0.1:8000".to_string());
    assert_eq!(share.node_index, 1);
    assert_eq!(share.owner, Addr::unchecked(MEMBER1));
    assert!(!share.verified);

    app.execute_contract(
        Addr::unchecked(MEMBER1),
        multisig_contract_addr.clone(),
        &Vote {
            proposal_id,
            vote: cw3::Vote::Yes,
        },
        &vec![],
    )
    .unwrap();

    for _ in 0..2 {
        app.execute_contract(
            Addr::unchecked(OWNER),
            coconut_dkg_contract_addr.clone(),
            &AdvanceEpochState {},
            &vec![],
        )
        .unwrap();
    }

    app.execute_contract(
        Addr::unchecked(MEMBER1),
        multisig_contract_addr.clone(),
        &Execute { proposal_id },
        &vec![],
    )
    .unwrap();

    let res: PagedVKSharesResponse = app
        .wrap()
        .query_wasm_smart(
            coconut_dkg_contract_addr,
            &GetVerificationKeys {
                limit: None,
                start_after: None,
            },
        )
        .unwrap();
    assert!(res.shares[0].verified);
}
