// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{coin, Coin};
use cw_multi_test::IntoBech32;
use cw_utils::PaymentError;
use nym_ecash_contract_common::EcashContractError;
use sylvia::{cw_multi_test::App as MtApp, multitest::App};

use crate::contract::sv::mt::{CodeId, NymEcashContractProxy};

const DENOM: &str = "unym";
const DEPOSIT_AMOUNT: u128 = 75_000_000;

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
            MockApi::default().addr_make("holding_acount").to_string(),
            MockApi::default().addr_make("multisig_addr").to_string(),
            MockApi::default().addr_make("group_addr").to_string(),
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

#[test]
fn wrong_deposit_amount() {
    let owner = "owner".into_bech32();

    let mtapp = MtApp::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, vec![Coin::new(1_000_000_000u128, DENOM)])
            .unwrap()
    });
    let app = App::new(mtapp);
    let code_id = CodeId::store_code(&app);
    let contract = code_id
        .instantiate(
            MockApi::default().addr_make("holding_account").to_string(),
            MockApi::default().addr_make("multisig_addr").to_string(),
            MockApi::default().addr_make("group_addr").to_string(),
            coin(DEPOSIT_AMOUNT, DENOM),
        )
        .call(&owner)
        .unwrap();

    let vk = "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK";

    // too little
    assert_eq!(
        contract
            .deposit_ticket_book_funds(vk.to_string())
            .with_funds(&[coin(1_000_000u128, DENOM)])
            .call(&owner)
            .unwrap_err(),
        EcashContractError::WrongAmount {
            received: coin(1_000_000u128, DENOM),
            amount: coin(DEPOSIT_AMOUNT, DENOM),
        }
    );

    // too much
    assert_eq!(
        contract
            .deposit_ticket_book_funds(vk.to_string())
            .with_funds(&[coin(100_000_000u128, DENOM)])
            .call(&owner)
            .unwrap_err(),
        EcashContractError::WrongAmount {
            received: coin(100_000_000u128, DENOM),
            amount: coin(DEPOSIT_AMOUNT, DENOM),
        }
    );
}

#[test]
fn correct_default_deposit_succeeds() {
    let owner = "owner".into_bech32();

    let mtapp = MtApp::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, vec![Coin::new(1_000_000_000u128, DENOM)])
            .unwrap()
    });
    let app = App::new(mtapp);
    let code_id = CodeId::store_code(&app);
    let contract = code_id
        .instantiate(
            MockApi::default().addr_make("holding_account").to_string(),
            MockApi::default().addr_make("multisig_addr").to_string(),
            MockApi::default().addr_make("group_addr").to_string(),
            coin(DEPOSIT_AMOUNT, DENOM),
        )
        .call(&owner)
        .unwrap();

    let vk = "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK";

    contract
        .deposit_ticket_book_funds(vk.to_string())
        .with_funds(&[coin(DEPOSIT_AMOUNT, DENOM)])
        .call(&owner)
        .unwrap();
}

#[test]
fn reduced_price_deposit_end_to_end() {
    let owner = "owner".into_bech32();
    let whitelisted = "whitelisted".into_bech32();
    let non_whitelisted = "non_whitelisted".into_bech32();
    let reduced_amount: u128 = 10_000_000;

    let mtapp = MtApp::new(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &whitelisted,
                vec![Coin::new(1_000_000_000u128, DENOM)],
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &non_whitelisted,
                vec![Coin::new(1_000_000_000u128, DENOM)],
            )
            .unwrap();
    });
    let app = App::new(mtapp);
    let code_id = CodeId::store_code(&app);
    let contract = code_id
        .instantiate(
            MockApi::default().addr_make("holding_account").to_string(),
            MockApi::default().addr_make("multisig_addr").to_string(),
            MockApi::default().addr_make("group_addr").to_string(),
            coin(DEPOSIT_AMOUNT, DENOM),
        )
        .call(&owner)
        .unwrap();

    let vk = "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK";

    // whitelist an address with a reduced price
    contract
        .set_reduced_deposit_price(whitelisted.to_string(), coin(reduced_amount, DENOM))
        .call(&owner)
        .unwrap();

    // whitelisted address can deposit at the reduced price
    contract
        .deposit_ticket_book_funds(vk.to_string())
        .with_funds(&[coin(reduced_amount, DENOM)])
        .call(&whitelisted)
        .unwrap();

    // whitelisted address can also deposit at the default price —
    // treated as a normal (non-reduced) deposit for statistics purposes
    contract
        .deposit_ticket_book_funds(vk.to_string())
        .with_funds(&[coin(DEPOSIT_AMOUNT, DENOM)])
        .call(&whitelisted)
        .unwrap();

    // whitelisted address is rejected when sending an amount that is
    // neither the reduced nor the default price
    assert_eq!(
        contract
            .deposit_ticket_book_funds(vk.to_string())
            .with_funds(&[coin(50_000_000, DENOM)])
            .call(&whitelisted)
            .unwrap_err(),
        EcashContractError::WrongAmount {
            received: coin(50_000_000, DENOM),
            amount: coin(reduced_amount, DENOM),
        }
    );

    // non-whitelisted address is rejected at the reduced amount
    assert_eq!(
        contract
            .deposit_ticket_book_funds(vk.to_string())
            .with_funds(&[coin(reduced_amount, DENOM)])
            .call(&non_whitelisted)
            .unwrap_err(),
        EcashContractError::WrongAmount {
            received: coin(reduced_amount, DENOM),
            amount: coin(DEPOSIT_AMOUNT, DENOM),
        }
    );

    // non-whitelisted address succeeds at the default amount
    contract
        .deposit_ticket_book_funds(vk.to_string())
        .with_funds(&[coin(DEPOSIT_AMOUNT, DENOM)])
        .call(&non_whitelisted)
        .unwrap();

    let stats = contract.get_deposits_statistics().unwrap();
    assert_eq!(stats.total_deposits_made, 3);
    assert_eq!(
        stats.total_deposited,
        coin(reduced_amount + DEPOSIT_AMOUNT * 2, DENOM)
    );
    // whitelisted depositing at default price + non-whitelisted = 2 default deposits
    assert_eq!(stats.total_deposits_made_with_default_price, 2);
    assert_eq!(
        stats.total_deposited_with_default_price,
        coin(DEPOSIT_AMOUNT * 2, DENOM)
    );
    assert_eq!(stats.total_deposits_made_with_custom_price, 1);
    assert_eq!(
        stats.total_deposited_with_custom_price,
        coin(reduced_amount, DENOM)
    );
}
