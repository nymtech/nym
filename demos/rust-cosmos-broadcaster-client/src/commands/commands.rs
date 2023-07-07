use nym_cli_commands::validator::mixnet::Mixnet;
use nym_crypto::generic_array::sequence;
use nym_sphinx_addressing::clients::Recipient;
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::nyxd::cosmwasm_client::types::SequenceResponse;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::tx_signer::TxSigner;
use nym_validator_client::signing::SignerData;
use cosmrs::bank::MsgSend;
use cosmrs::rpc::{HttpClient, Id};
use cosmrs::tx::Msg;
use cosmrs::{tx, AccountId, Coin, Denom};
use bip39; 
use bs58; 
use nym_sdk::mixnet::{self, MixnetClient};
use serde::{Deserialize, Serialize};
// use serde_json::Result; 

#[derive(Deserialize, Serialize)]
struct SequenceRequestData<'a> {
    validator: &'a str, 
    signer_address: AccountId, 
    request_type: String
}

struct SequenceResponseData {
    sequence_response: SequenceResponse, 
    chain_id: Id
}

pub async fn offline_sign(mnemonic: bip39::Mnemonic, to: AccountId, client: &mut MixnetClient , sp_address: Recipient) -> String {

    // TODO take coin amount from function args, + load network vars from config file. 
    let prefix = "n";
    let denom: Denom = "unym".parse().unwrap();
    let validator = "https://qwerty-validator.qa.nymte.ch";

    let signer = DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic);
    let signer_address = signer.try_derive_accounts().unwrap()[0].address().clone();

    // local 'client' ONLY signing messages
    let tx_signer = TxSigner::new(signer);

/////////////// to go on server side
    // possibly remote client that doesn't do ANY signing
    // (only broadcasts + queries for sequence numbers)
    // let broadcaster = HttpClient::new(validator).unwrap();
    // // get sequence information and chain ID from the service 
    // // add sdk, send req for sequence 
    // // get signer information
// let sequence_response = broadcaster.get_sequence(&signer_address).await.unwrap();
    // let chain_id = broadcaster.get_chain_id().await.unwrap();
/////////////// 
 
    let message = SequenceRequestData{
        validator, 
        signer_address,
        request_type: String::from("sequence_request")
    }; 
    // send req to client 
    client.send_str(sp_address, &serde_json::to_string(&message).unwrap()).await;
    let res = client.wait_for_messages().await; 
    for i in res.unwrap().iter() {
        println!("{:#?}", i.message); 
    }
    // parse json of res to get signer_data and chain_id, store in SeqResData struct 

    todo!()

/* 
    // use the response to create SignerData instance 
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
*/
}

pub async fn send_tx(base58_tx: String, sp_address: Recipient, client: &mut MixnetClient) -> Option<Vec<mixnet::ReconstructedMessage>> {
    client.send_str(sp_address, &base58_tx).await; 
    println!("\nWaiting for reply\n");
    let res = client.wait_for_messages().await; 
    res 
}
