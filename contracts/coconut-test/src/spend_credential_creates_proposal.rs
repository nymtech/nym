use crate::helpers::*;
use coconut_bandwidth::error::ContractError;
use coconut_bandwidth_contract_common::{
    msg::{
        ExecuteMsg as CoconutBandwidthExecuteMsg, InstantiateMsg as CoconutBandwidthInstantiateMsg,
    },
    spend_credential::SpendCredentialData,
};
use config::defaults::MIX_DENOM;
use cosmwasm_std::{coins, Addr, Coin, Decimal};
use cw4_group::msg::InstantiateMsg as GroupInstantiateMsg;
use cw_multi_test::Executor;
use cw_utils::{Duration, Threshold};
use multisig_contract_common::msg::InstantiateMsg as MultisigInstantiateMsg;

#[test]
fn spend_credential_creates_proposal() {
    let init_funds = coins(10, MIX_DENOM.base);
    let mut app = mock_app(&init_funds);
    let pool_addr = String::from(POOL_CONTRACT);

    let group_code_id = app.store_code(contract_group());
    let msg = GroupInstantiateMsg {
        admin: Some(OWNER.to_string()),
        members: vec![],
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
        group_addr: group_contract_addr.into_string(),
        threshold: Threshold::AbsolutePercentage {
            percentage: Decimal::from_ratio(2u128, 3u128),
        },
        max_voting_period: Duration::Height(1000),
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

    let coconut_bandwidth_code_id = app.store_code(contract_bandwidth());
    let msg = CoconutBandwidthInstantiateMsg {
        multisig_addr: multisig_contract_addr.to_string(),
        pool_addr,
    };
    let coconut_bandwidth_contract_addr = app
        .instantiate_contract(
            coconut_bandwidth_code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "coconut bandwidth",
            None,
        )
        .unwrap();

    let msg = MigrateMsg {
        coconut_bandwidth_address: coconut_bandwidth_contract_addr.to_string(),
    };
    app.migrate_contract(
        Addr::unchecked(OWNER),
        multisig_contract_addr,
        &msg,
        multisig_code_id,
    )
    .unwrap();

    let msg = CoconutBandwidthExecuteMsg::SpendCredential {
        data: SpendCredentialData::new(
            Coin::new(1, MIX_DENOM.base),
            String::from("blinded_serial_number"),
            String::from("gateway_cosmos_address"),
        ),
    };
    let res = app
        .execute_contract(
            Addr::unchecked(OWNER),
            coconut_bandwidth_contract_addr.clone(),
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
    assert_eq!(1, proposal_id);

    // Trying with the same blinded serial number will detect the double spend attempt
    let err = app
        .execute_contract(
            Addr::unchecked(OWNER),
            coconut_bandwidth_contract_addr.clone(),
            &msg,
            &vec![],
        )
        .unwrap_err();
    assert_eq!(
        ContractError::DuplicateBlindedSerialNumber,
        err.downcast().unwrap()
    );

    let msg = CoconutBandwidthExecuteMsg::SpendCredential {
        data: SpendCredentialData::new(
            Coin::new(1, MIX_DENOM.base),
            String::from("blinded_serial_number2"),
            String::from("gateway_cosmos_address"),
        ),
    };
    let res = app
        .execute_contract(
            Addr::unchecked(OWNER),
            coconut_bandwidth_contract_addr.clone(),
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
    assert_eq!(2, proposal_id);
}
