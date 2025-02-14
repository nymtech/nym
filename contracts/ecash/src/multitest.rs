// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{coin, Coin};
use cw_multi_test::IntoBech32;
use cw_utils::PaymentError;
use nym_ecash_contract_common::EcashContractError;
use sylvia::{cw_multi_test::App as MtApp, multitest::App};

use crate::contract::sv::mt::{CodeId, NymEcashContractProxy};

#[test]
fn invalid_deposit() {
    let owner = "owner".into_bech32();
    let denom = "unym";

    let mtapp = MtApp::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &owner,
                vec![
                    Coin::new(10000000u32, denom),
                    Coin::new(10000000u32, "some_denom"),
                ],
            )
            .unwrap()
    });
    let app = App::new(mtapp);

    let code_id = CodeId::store_code(&app);

    let contract = code_id
        .instantiate(
            "holding_acount".to_string(),
            "multisig_addr".to_string(),
            "group_addr".to_string(),
            coin(75000000, denom),
        )
        .call(&owner)
        .unwrap();

    let verification_key = "Verification key";

    assert_eq!(
        contract
            .deposit_ticket_book_funds(verification_key.to_string(),)
            .call(&owner)
            .unwrap_err(),
        EcashContractError::InvalidDeposit(PaymentError::NoFunds {})
    );

    let coin = Coin::new(1000000u32, denom.to_string());
    let second_coin = Coin::new(1000000u32, "some_denom");

    assert_eq!(
        contract
            .deposit_ticket_book_funds(verification_key.to_string(),)
            .with_funds(&[coin, second_coin.clone()])
            .call(&owner)
            .unwrap_err(),
        EcashContractError::InvalidDeposit(PaymentError::MultipleDenoms {})
    );

    assert_eq!(
        contract
            .deposit_ticket_book_funds(verification_key.to_string(),)
            .with_funds(&[second_coin])
            .call(&owner)
            .unwrap_err(),
        EcashContractError::InvalidDeposit(PaymentError::MissingDenom(denom.to_string()))
    );
}
