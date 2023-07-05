use cosmrs::bip32::secp256k1::elliptic_curve::generic_array::sequence;
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::tx_signer::TxSigner;
use nym_validator_client::signing::SignerData;
use cosmrs::bank::MsgSend;
use cosmrs::rpc::{self, HttpClient};
use cosmrs::tx::Msg;
use cosmrs::{tx, AccountId, Coin, Denom};
use bip39; 
use bs58; 
use nym_sdk::mixnet;

pub async fn offline_sign(mnemonic: bip39::Mnemonic, to: AccountId) -> String {

    // TODO take coin amount from function args, + load network vars from config file. 
    let prefix = "n";
    let denom: Denom = "unym".parse().unwrap();
    let validator = "https://qwerty-validator.qa.nymte.ch";

    let signer = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);
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
        to_address: to.clone(),
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
        100000,
    );

    let tx_raw = tx_signer
        .sign_direct(&signer_address, vec![send_msg], fee, memo, signer_data)
        .unwrap();

    let tx_bytes = tx_raw.to_bytes().unwrap();
    let base58_tx_bytes = bs58::encode(tx_bytes).into_string();
    base58_tx_bytes

}

pub async fn send_tx(base58_tx: String, sp_address: String) -> Option<Vec<mixnet::ReconstructedMessage>> /*String*/ {
    // 1. decode base58 -> vec<u8> 
    println!("this is where we decode the base58 string"); 

    // 2. get nym client address 
    let client = mixnet::MixnetClientBuilder::new_ephemeral()
        .build()
        .await
        .unwrap();

    let mut client = client.connect_to_mixnet().await.unwrap();
    
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // 3. send message w sdk to broadcaster who will do: 
    /* 
        // broadcast the tx
        let res = rpc::Client::broadcast_tx_commit(&broadcaster, tx_bytes.into())
        .await
        .unwrap();
     */
    // client.send_str(sp_address, signedtx); 
    // println!("Waiting for message");
    let res = client.wait_for_messages().await; 

    // disconnect client 
    // return the res to return to main thread 
    client.disconnect().await;
    // String::from("your tx hash")
    res 

}
