use crate::helpers::*;
use cosmwasm_std::{coins, Addr, Coin, Decimal};
use cw_multi_test::Executor;
use cw_utils::{Duration, Threshold};
use nym_coconut_bandwidth::error::ContractError;
use nym_coconut_bandwidth_contract_common::{
    msg::{
        ExecuteMsg as CoconutBandwidthExecuteMsg, InstantiateMsg as CoconutBandwidthInstantiateMsg,
    },
    spend_credential::SpendCredentialData,
};
use nym_group_contract_common::msg::InstantiateMsg as GroupInstantiateMsg;
use nym_multisig_contract_common::msg::InstantiateMsg as MultisigInstantiateMsg;

pub const TEST_COIN_DENOM: &str = "unym";
pub const TEST_COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub const TEST_COCONUT_DKG_CONTRACT_ADDRESS: &str = "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";

#[test]
fn spend_credential_creates_proposal() {
    let init_funds = coins(10, TEST_COIN_DENOM);
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
        executor: None,
        proposal_deposit: None,
        max_voting_period: Duration::Height(1000),
        coconut_bandwidth_contract_address: TEST_COCONUT_BANDWIDTH_CONTRACT_ADDRESS.to_string(),
        coconut_dkg_contract_address: TEST_COCONUT_DKG_CONTRACT_ADDRESS.to_string(),
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
        mix_denom: TEST_COIN_DENOM.to_string(),
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
        coconut_dkg_address: "dkg-address".to_string(),
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
            Coin::new(1, TEST_COIN_DENOM),
            String::from("blinded_serial_number"),
            String::from("gateway_cosmos_address"),
        ),
    };
    let res = app
        .execute_contract(
            Addr::unchecked(OWNER),
            coconut_bandwidth_contract_addr.clone(),
            &msg,
            &[],
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
            &[],
        )
        .unwrap_err();
    assert_eq!(
        ContractError::DuplicateBlindedSerialNumber,
        err.downcast().unwrap()
    );

    let msg = CoconutBandwidthExecuteMsg::SpendCredential {
        data: SpendCredentialData::new(
            Coin::new(1, TEST_COIN_DENOM),
            String::from("blinded_serial_number2"),
            String::from("gateway_cosmos_address"),
        ),
    };
    let res = app
        .execute_contract(
            Addr::unchecked(OWNER),
            coconut_bandwidth_contract_addr,
            &msg,
            &[],
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
