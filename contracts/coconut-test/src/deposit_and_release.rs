// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::*;
use coconut_bandwidth::error::ContractError;
use coconut_bandwidth_contract_common::{
    deposit::DepositData,
    msg::{ExecuteMsg, InstantiateMsg},
};
use cosmwasm_std::{coins, Addr};
use cw_controllers::AdminError;
use cw_multi_test::Executor;

const TEST_MIX_DENOM: &str = "unym";

#[test]
fn deposit_and_release() {
    let init_funds = coins(10, TEST_MIX_DENOM);
    let deposit_funds = coins(1, TEST_MIX_DENOM);
    let release_funds = coins(2, TEST_MIX_DENOM);
    let mut app = mock_app(&init_funds);
    let multisig_addr = String::from(MULTISIG_CONTRACT);
    let pool_addr = String::from(POOL_CONTRACT);
    let random_addr = String::from(RANDOM_ADDRESS);

    let code_id = app.store_code(contract_bandwidth());
    let msg = InstantiateMsg {
        multisig_addr: multisig_addr.clone(),
        pool_addr: pool_addr.clone(),
        mix_denom: TEST_MIX_DENOM.to_string(),
    };
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "bandwidth",
            None,
        )
        .unwrap();

    let msg = ExecuteMsg::DepositFunds {
        data: DepositData::new(
            String::from("info"),
            String::from("id"),
            String::from("enc"),
        ),
    };
    app.execute_contract(
        Addr::unchecked(OWNER),
        contract_addr.clone(),
        &msg,
        &deposit_funds,
    )
    .unwrap();

    // try to release more then it's in the contract
    let msg = ExecuteMsg::ReleaseFunds {
        funds: release_funds[0].clone(),
    };
    let err = app
        .execute_contract(
            Addr::unchecked(multisig_addr.clone()),
            contract_addr.clone(),
            &msg,
            &[],
        )
        .unwrap_err();
    assert_eq!(ContractError::NotEnoughFunds, err.downcast().unwrap());

    // try to call release from non-admin
    let msg = ExecuteMsg::ReleaseFunds {
        funds: deposit_funds[0].clone(),
    };
    let err = app
        .execute_contract(
            Addr::unchecked(random_addr),
            contract_addr.clone(),
            &msg,
            &[],
        )
        .unwrap_err();
    assert_eq!(
        ContractError::Admin(AdminError::NotAdmin {}),
        err.downcast().unwrap()
    );

    let msg = ExecuteMsg::ReleaseFunds {
        funds: deposit_funds[0].clone(),
    };
    app.execute_contract(
        Addr::unchecked(multisig_addr),
        contract_addr.clone(),
        &msg,
        &[],
    )
    .unwrap();
    let pool_bal = app.wrap().query_balance(pool_addr, TEST_MIX_DENOM).unwrap();
    assert_eq!(pool_bal, deposit_funds[0]);
}
