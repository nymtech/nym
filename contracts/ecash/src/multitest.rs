// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Coin};
use cw_utils::PaymentError;
use nym_ecash_contract_common::EcashContractError;
use sylvia::{cw_multi_test::App as MtApp, multitest::App};

use crate::contract::multitest_utils::CodeId;

#[test]
fn invalid_deposit() {
    let owner = "owner";
    let denom = "unym";

    let mtapp = MtApp::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(owner),
                vec![
                    Coin::new(10000000, denom),
                    Coin::new(10000000, "some_denom"),
                ],
            )
            .unwrap()
    });
    let app = App::new(mtapp);

    let code_id = CodeId::store_code(&app);

    let contract = code_id
        .instantiate(
            "multisig_addr".to_string(),
            "group_addr".to_string(),
            denom.to_string(),
        )
        .call(owner)
        .unwrap();

    let verification_key = "Verification key";

    assert_eq!(
        contract
            .deposit_ticket_book_funds(verification_key.to_string(),)
            .call(owner)
            .unwrap_err(),
        EcashContractError::InvalidDeposit(PaymentError::NoFunds {})
    );

    let coin = Coin::new(1000000, denom.to_string());
    let second_coin = Coin::new(1000000, "some_denom");

    assert_eq!(
        contract
            .deposit_ticket_book_funds(verification_key.to_string(),)
            .with_funds(&[coin, second_coin.clone()])
            .call(owner)
            .unwrap_err(),
        EcashContractError::InvalidDeposit(PaymentError::MultipleDenoms {})
    );

    assert_eq!(
        contract
            .deposit_ticket_book_funds(verification_key.to_string(),)
            .with_funds(&[second_coin])
            .call(owner)
            .unwrap_err(),
        EcashContractError::InvalidDeposit(PaymentError::MissingDenom(denom.to_string()))
    );
}
