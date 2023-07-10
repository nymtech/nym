// use nym_cli_commands::validator::mixnet::Mixnet;
// use nym_crypto::generic_array::sequence;
// use nym_sphinx_addressing::clients::Recipient;
use nym_validator_client::nyxd::CosmWasmClient;
// use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
// use nym_validator_client::signing::tx_signer::TxSigner;
// use nym_validator_client::signing::SignerData;
// use cosmrs::bank::MsgSend;
use cosmrs::rpc::{HttpClient, Id, Client};
// use cosmrs::tx::Msg;
use cosmrs::{tx, AccountId, Coin, Denom, tendermint};
// use bip39; 
use bs58; 
use nym_sdk::mixnet::{self, MixnetClient};
use serde::{Deserialize, Serialize};
use crate::commands::reqres::{SequenceRequest, SequenceRequestResponse};

pub async fn get_sequence(validator: String, signer_address: AccountId) -> SequenceRequestResponse {

    /*
      TODO create broadcaster in different fn and build on setup - pass to both fns as arg 
     */
    let broadcaster = HttpClient::new(validator.as_str()).unwrap();

    // get signer information
    let sequence = broadcaster.get_sequence(&signer_address).await.unwrap();
    let chain_id: tendermint::chain::Id = broadcaster.get_chain_id().await.unwrap();
    let res = SequenceRequestResponse { account_number: sequence.account_number, sequence: sequence.sequence, chain_id };
    res  
}

pub async fn broadcast(base58_tx_bytes: String) -> String {
    todo!();

    /*
      TODO create broadcaster in different fn and build on setup - pass to both fns as arg 
     */

    // decode the base58 tx to vec<u8>
    // let tx_bytes = bs58::decode(base58_tx_bytes);  

    // // create instance of Transaction struct w tx_bytes - tx_to_broadcast 

    // // broadcast the tx
    // let res = Client::broadcast_tx_commit(&broadcaster, tx_to_broadcast)
    // .await
    // .unwrap();

    // let placeholder = String::from("palceholder"); 
    // placeholder 

}