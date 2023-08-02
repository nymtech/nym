// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::bank::MsgSend;
use cosmrs::rpc::HttpClient;
use cosmrs::tx::Msg;
use cosmrs::{tx, AccountId, Coin, Denom};
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::tx_signer::TxSigner;
use nym_validator_client::signing::SignerData;

// run with: cargo run --example offline_signing --features=nyxd-client
#[tokio::main]
async fn main() {
    let prefix = "n";
    let denom: Denom = "unym".parse().unwrap();
    let signer_mnemonic: bip39::Mnemonic = "<MNEMONIC WITH FUNDS HERE>".parse().unwrap();
    let validator = "https://qwerty-validator.qa.nymte.ch";
    let to_address: AccountId = "n19kdst4srf76xgwe55jg32mpcpcyf6aqgp6qrdk".parse().unwrap();

    let signer = DirectSecp256k1HdWallet::from_mnemonic(prefix, signer_mnemonic);
    let signer_address = signer.try_derive_accounts().unwrap()[0].address().clone();

    // local 'client' ONLY signing messages
    let tx_signer = TxSigner::new(signer);

    // possibly remote client that doesn't do ANY signing
    // (only broadcasts + queries for sequence numbers)
    let broadcaster = HttpClient::new(validator).unwrap();

    // get signer information
    let sequence_response = broadcaster.get_sequence(&signer_address).await.unwrap();
    let chain_id = broadcaster.get_chain_id().await.unwrap();
    let signer_data = SignerData::new_from_sequence_response(sequence_response, chain_id);

    // create (and sign) the send message
    let amount = vec![Coin {
        denom: denom.clone(),
        amount: 12345u32.into(),
    }];

    let send_msg = MsgSend {
        from_address: signer_address.clone(),
        to_address: to_address.clone(),
        amount,
    }
    .to_any()
    .unwrap();

    let memo = "example memo";
    let fee = tx::Fee::from_amount_and_gas(
        Coin {
            denom,
            amount: 2500u32.into(),
        },
        100000u32,
    );

    let tx_raw = tx_signer
        .sign_direct(&signer_address, vec![send_msg], fee, memo, signer_data)
        .unwrap();
    let tx_bytes = tx_raw.to_bytes().unwrap();

    // compare balances from before and after the tx
    let before = broadcaster
        .get_balance(&to_address, "unym".to_string())
        .await
        .unwrap()
        .unwrap();

    // broadcast the tx
    let res = tendermint_rpc::client::Client::broadcast_tx_commit(&broadcaster, tx_bytes)
        .await
        .unwrap();

    let after = broadcaster
        .get_balance(&to_address, "unym".to_string())
        .await
        .unwrap()
        .unwrap();

    println!("transaction result: {res:?})");
    println!("balance before: {before}");
    println!("balance after:  {after}");
}
