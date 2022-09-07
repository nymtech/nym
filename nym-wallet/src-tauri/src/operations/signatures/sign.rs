use std::str::FromStr;

use crate::error::BackendError;
use crate::state::WalletState;
use cosmrs::crypto::secp256k1::{Signature, VerifyingKey};
use cosmrs::crypto::PublicKey;
use k256::ecdsa::signature::Verifier;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct SignatureOutputJson {
    pub account_id: String,
    pub public_key: PublicKey,
    pub signature: String,
}

#[tauri::command]
pub async fn sign(
    message: String,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let wallet = client.nymd.signer();
    let derived_accounts = wallet.try_derive_accounts()?;
    match derived_accounts.first() {
        Some(account) => {
            log::info!("<<< Signing message");
            let signature = wallet.sign_raw_with_account(account, message.as_bytes())?;
            let signature_as_hex_string = signature.to_string();
            let output = SignatureOutputJson {
                account_id: account.address().to_string(),
                public_key: account.public_key(),
                signature: signature_as_hex_string.to_string(),
            };
            log::info!(">>> Signing data {}", json!(output),);
            Ok(signature_as_hex_string)
        }
        None => {
            log::error!(">>> Unable to derive account");
            Err(BackendError::SignatureError(
                "unable to derive account".to_string(),
            ))
        }
    }
}

#[tauri::command]
pub async fn verify(
    public_key_as_json: String,
    signature_as_hex: String,
    message: String,
    _state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    let public_key = PublicKey::from_json(&public_key_as_json)?;
    if public_key.type_url() != PublicKey::SECP256K1_TYPE_URL {
        return Err(BackendError::SignatureError(
            "Sorry, we only support secp256k1 public keys at the moment".to_string(),
        ));
    }

    log::info!("<<< Verifying signature [{}]", signature_as_hex);
    let verifying_key = VerifyingKey::from_sec1_bytes(&public_key.to_bytes())?;
    let signature = Signature::from_str(&signature_as_hex)?;
    let message_as_bytes = message.into_bytes();
    Ok(verifying_key
        .verify(&message_as_bytes, &signature)
        .map_err(|e| {
            log::error!(">>> Verification failed, wrong signature");
            e
        })?)
}
