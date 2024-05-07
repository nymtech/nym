// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Coin;
use cw_utils::PaymentError;
use sylvia::multitest::App;

use crate::{contract::multitest_utils::CodeId, errors::ContractError};

#[test]
fn invalid_deposit() {
    let app = App::default();
    let owner = "owner";
    let denom = "unym";
    let code_id = CodeId::store_code(&app);

    let contract = code_id
        .instantiate(
            "multisig_addr".to_string(),
            "group_addr".to_string(),
            denom.to_string(),
        )
        .call(owner)
        .unwrap();

    let deposit_info = "Deposit info";
    let verification_key = "Verification key";

    assert_eq!(
        contract
            .deposit_funds(deposit_info.to_string(), verification_key.to_string(),)
            .call(owner)
            .unwrap_err(),
        ContractError::InvalidDeposit(PaymentError::NoFunds {})
    );

    let coin = Coin::new(1000000, denom.to_string());
    let second_coin = Coin::new(1000000, "some_denom");

    assert_eq!(
        contract
            .deposit_funds(deposit_info.to_string(), verification_key.to_string(),)
            .with_funds(&[coin, second_coin.clone()])
            .call(owner)
            .unwrap_err(),
        ContractError::InvalidDeposit(PaymentError::MultipleDenoms {})
    );

    assert_eq!(
        contract
            .deposit_funds(deposit_info.to_string(), verification_key.to_string(),)
            .with_funds(&[second_coin])
            .call(owner)
            .unwrap_err(),
        ContractError::InvalidDeposit(PaymentError::MissingDenom(denom.to_string()))
    );
}