use crate::{DEFAULT_DENOM, DEFAULT_PREFIX, DEFAULT_VALIDATOR_RPC};
use bip39;
use bs58;
use cosmrs::bank::MsgSend;
use cosmrs::tx::Msg;
use cosmrs::{tx, AccountId, Coin, Denom};
use nym_sdk::mixnet::MixnetClient;
use nym_sphinx_addressing::clients::Recipient;
use nym_validator_client::nyxd::cosmwasm_client::types;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
use nym_validator_client::signing::tx_signer::TxSigner;
use nym_validator_client::signing::SignerData;

pub async fn offline_sign(
    mnemonic: bip39::Mnemonic,
    to: AccountId,
    client: &mut MixnetClient,
    sp_address: Recipient,
) -> anyhow::Result<String> {
    let denom: Denom = DEFAULT_DENOM.parse().unwrap();
    let signer = DirectSecp256k1HdWallet::from_mnemonic(DEFAULT_PREFIX, mnemonic.clone());
    let signer_address = signer.try_derive_accounts().unwrap()[0].address().clone();

    // local 'client' ONLY signing messages
    let tx_signer = TxSigner::new(signer);

    // sequence request type
    let message = crate::SequenceRequest {
        validator: DEFAULT_VALIDATOR_RPC.to_owned(), // rpc endpoint for broadcaster to use
        signer_address: signer_address.clone(),      // our (sender) address, derived from mnemonic
    };

    // send req to service via the mixnet
    client
        .send_str(sp_address, &serde_json::to_string(&message).unwrap())
        .await;

    // listen for response from service
    let sp_response = crate::listen_and_parse_response(client).await?;

    // match JSON -> ResponseType
    let res = match sp_response {
        crate::ResponseTypes::Sequence(request) => {
            println!(
                "got a response to the chain sequence request. using this to sign our tx offline"
            );

            // use the response to create SignerData instance
            let sequence_response = types::SequenceResponse {
                account_number: request.account_number,
                sequence: request.sequence,
            };
            let signer_data =
                SignerData::new_from_sequence_response(sequence_response, request.chain_id);

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
            // encode tx bytes as base58 for ease of logging + copying for user
            let base58_tx_bytes = bs58::encode(tx_bytes).into_string();
            base58_tx_bytes
        }
        _ => String::from("unexpected response"),
    };
    Ok(res)
}

pub async fn send_tx(
    base58_tx: String,
    sp_address: Recipient,
    client: &mut MixnetClient,
) -> anyhow::Result<(String, bool)> {
    let broadcast_request = crate::BroadcastRequest {
        base58_tx_bytes: base58_tx,
    };

    // send broadcast request containing base58 encoded signed tx to service via mixnet
    client
        .send_str(
            sp_address,
            &serde_json::to_string(&broadcast_request).unwrap(),
        )
        .await;
    println!("Waiting for reply");

    // again, listen for response and parse accordingly
    let sp_response = crate::listen_and_parse_response(client).await?;

    let res = match sp_response {
        crate::ResponseTypes::Broadcast(response) => {
            let broadcast_response = crate::BroadcastResponse {
                tx_hash: response.tx_hash,
                success: response.success,
            };
            (broadcast_response.tx_hash, broadcast_response.success)
        }
        _ => (
            String::from("Got strange incoming response, couldn't match"),
            false,
        ),
    };
    Ok(res)
}
