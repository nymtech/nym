use coconut_bandwidth_contract_common::{
    msg::{
        ExecuteMsg as CoconutBandwidthExecuteMsg, InstantiateMsg as CoconutBandwidthInstantiateMsg,
    },
    spend_credential::SpendCredentialData,
};
use coconut_test::helpers::*;
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

    let code_id = app.store_code(contract_group());
    let msg = GroupInstantiateMsg {
        admin: None,
        members: vec![],
    };
    let group_contract_addr = app
        .instantiate_contract(code_id, Addr::unchecked(OWNER), &msg, &[], "group", None)
        .unwrap();

    let code_id = app.store_code(contract_multisig());
    let msg = MultisigInstantiateMsg {
        group_addr: group_contract_addr.into_string(),
        threshold: Threshold::AbsolutePercentage {
            percentage: Decimal::from_ratio(2u128, 3u128),
        },
        max_voting_period: Duration::Height(1000),
    };
    let multisig_contract_addr = app
        .instantiate_contract(code_id, Addr::unchecked(OWNER), &msg, &[], "multisig", None)
        .unwrap();

    let code_id = app.store_code(contract_bandwidth());
    let msg = CoconutBandwidthInstantiateMsg {
        multisig_addr: multisig_contract_addr.into_string(),
        pool_addr,
    };
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "coconut bandwidth",
            None,
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
        .execute_contract(Addr::unchecked(OWNER), contract_addr.clone(), &msg, &vec![])
        .unwrap();

    println!("Events: {:?}", res.events);
    assert!(false);
}
