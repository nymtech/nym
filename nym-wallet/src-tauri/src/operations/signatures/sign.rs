use crate::error::BackendError;
use crate::state::WalletState;
use cosmrs::crypto::secp256k1::{Signature, VerifyingKey};
use cosmrs::crypto::PublicKey;
use cosmrs::AccountId;
use k256::ecdsa::signature::Verifier;
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::signing::signer::OfflineSigner;
use serde::Serialize;
use serde_json::json;
use std::str::FromStr;

#[derive(Debug, Serialize)]
pub struct SignatureOutputJson {
    pub account_id: String,
    pub public_key: PublicKey,
    pub signature_as_hex: String,
}

#[tauri::command]
pub async fn sign(
    message: String,
    state: tauri::State<'_, WalletState>,
) -> Result<String, BackendError> {
    let guard = state.read().await;
    let client = guard.current_client()?;
    let derived_accounts = client.nyxd.get_accounts()?;
    let account = derived_accounts.first().ok_or_else(|| {
        log::error!(">>> Unable to derive account");
        BackendError::SignatureError("unable to derive account".to_string())
    })?;

    log::info!("<<< Signing message");
    let signature = client
        .nyxd
        .sign_raw_with_account(account, message.as_bytes())?;
    let output = SignatureOutputJson {
        account_id: account.address().to_string(),
        public_key: account.public_key(),
        signature_as_hex: signature.to_string(),
    };
    let output_json = json!(output).to_string();
    log::info!(">>> Signing data {}", output_json);
    Ok(output_json)
}

async fn get_pubkey_from_account_address(
    address: &AccountId,
    state: &tauri::State<'_, WalletState>,
) -> Result<PublicKey, BackendError> {
    log::info!("Getting public key for address {} from chain...", address);
    let guard = state.read().await;
    let client = guard.current_client()?;
    let account = client.nyxd.get_account(address).await?.ok_or_else(|| {
        log::error!("No account associated with address {}", address);
        BackendError::SignatureError(format!("No account associated with address {address}"))
    })?;
    let base_account = account.try_get_base_account()?;

    base_account.pubkey.ok_or_else(|| {
        log::error!("No pubkey found for address {}", address);
        BackendError::SignatureError(format!("No pubkey found for address {address}"))
    })
}

enum VerifyInputKind {
    PublicKey(PublicKey),
    AccountAddress(String),
    CurrentAccountAddress,
}

impl TryFrom<Option<String>> for VerifyInputKind {
    type Error = BackendError;

    fn try_from(value: Option<String>) -> Result<Self, Self::Error> {
        let key = match value {
            Some(key) => key,
            None => return Ok(VerifyInputKind::CurrentAccountAddress),
        };
        if key.trim().is_empty() {
            return Err(BackendError::SignatureError(
                "Please ensure the public key or address is not empty or whitespace".to_string(),
            ));
        }
        let account_id = AccountId::from_str(&key);
        let key_from_json = PublicKey::from_json(&key);
        if account_id.is_err() && key_from_json.is_err() {
            return Err(BackendError::SignatureError(
                "Please ensure the public key or address is valid".to_string(),
            ));
        }
        if let Ok(k) = key_from_json {
            Ok(VerifyInputKind::PublicKey(k))
        } else {
            Ok(VerifyInputKind::AccountAddress(key))
        }
    }
}

#[tauri::command]
pub async fn verify(
    public_key_as_json_or_account_address: Option<String>,
    signature_as_hex: String,
    message: String,
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    let public_key = match VerifyInputKind::try_from(public_key_as_json_or_account_address)? {
        VerifyInputKind::PublicKey(key) => key,
        VerifyInputKind::AccountAddress(address) => {
            // get public key from the given account address
            get_pubkey_from_account_address(&AccountId::from_str(&address)?, &state).await?
        }
        VerifyInputKind::CurrentAccountAddress => {
            // get public key from current account address
            let guard = state.read().await;
            let client = guard.current_client()?;
            let address = &client.nyxd.address();
            get_pubkey_from_account_address(address, &state).await?
        }
    };

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
