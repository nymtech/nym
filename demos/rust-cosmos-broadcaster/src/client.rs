use nym_sphinx_addressing::clients::Recipient;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::tx_signer::TxSigner;
use nym_validator_client::signing::SignerData;
use nym_validator_client::nyxd::cosmwasm_client::types;
use cosmrs::bank::MsgSend;
use cosmrs::tx::Msg;
use cosmrs::{tx, AccountId, Coin, Denom};
use bip39; 
use bs58; 
use nym_sdk::mixnet::{self, MixnetClient, ReconstructedMessage};
use crate::{DEFAULT_VALIDATOR_RPC, DEFAULT_DENOM, DEFAULT_PREFIX};

// TODO take coin amount from function args
pub async fn offline_sign(mnemonic: bip39::Mnemonic, to: AccountId, client: &mut MixnetClient , sp_address: Recipient) -> String {

    let denom: Denom =  DEFAULT_DENOM.parse().unwrap(); 
    let signer = DirectSecp256k1HdWallet::from_mnemonic(DEFAULT_PREFIX, mnemonic.clone());
    let signer_address = signer.try_derive_accounts().unwrap()[0].address().clone();

    // local 'client' ONLY signing messages
    let tx_signer = TxSigner::new(signer);

    let message = crate::SequenceRequest{
        validator: DEFAULT_VALIDATOR_RPC.to_owned(), 
        signer_address,
    }; 

    // send req to client 
    client.send_str(sp_address, &serde_json::to_string(&message).unwrap()).await;

    // handle incoming message - we presume its a reply from the SP 
    let mut message: Vec<ReconstructedMessage> = Vec::new(); 

    // get the actual message - discard the empty vec sent along with the SURB topup request  
    while let Some(new_message) = client.wait_for_messages().await {
       if new_message.is_empty() {
        continue;
       } println!("got a response"); 
        message = new_message;
       break  
    }

    // parse vec<u8> -> JSON String 
    let mut parsed = String::new(); 
    for r in message.iter() {
        parsed = String::from_utf8(r.message.clone()).unwrap();
        break
    };  
    let sp_response: crate::ResponseTypes = serde_json::from_str(&parsed).unwrap(); 

    // match JSON -> ResponseType 
    let res = match sp_response {
        crate::ResponseTypes::Sequence(request) => {
            println!("got a response to the chain sequence request. using this to sign our tx offline"); 

            // use the response to create SignerData instance 
            let sequence_response = types::SequenceResponse {
                account_number: request.account_number, 
                sequence: request.sequence
            }; 
            let signer_data = SignerData::new_from_sequence_response( sequence_response, request.chain_id);

            // create (and sign) the send message
            let amount = vec![Coin {
                denom: denom.clone(),
                amount: 12345u32.into(),
            }];

            // TODO there must be a better way of doing this instead of re-generating the signer address from the mnemonic twice 
            let signer = DirectSecp256k1HdWallet::from_mnemonic(DEFAULT_PREFIX, mnemonic.clone());
            let signer_address = signer.try_derive_accounts().unwrap()[0].address().clone();
            
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
        }, 
        // TODO make this a proper error 
        _ => { println!("weird response"); String::from("placeholder error") }
    };

    res 

}

pub async fn send_tx(base58_tx: String, sp_address: Recipient, client: &mut MixnetClient) -> Option<Vec<mixnet::ReconstructedMessage>> {

    let broadcast_request = crate::BroadcastRequest {
        base58_tx_bytes: base58_tx
    };

    client.send_str(sp_address, &serde_json::to_string(&broadcast_request).unwrap()).await; 

    println!("\nWaiting for reply\n");
    
     
    client.wait_for_messages().await 
}
