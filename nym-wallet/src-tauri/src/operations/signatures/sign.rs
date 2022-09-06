use crate::error::BackendError;
use crate::state::WalletState;

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
            log::info!(
                "<<< Signing message with account {} and public key {:#?}",
                account.address(),
                account.public_key()
            );
            let signature = wallet.sign_raw_with_account(account, message.as_bytes())?;
            let signature_as_hex_string = signature.to_string();
            log::info!(">>> Signature: {}", &signature_as_hex_string);
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
