use std::path::Path;
use std::{fs, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    error::{BackendError, Result},
    state::State,
};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::client::key_manager::ClientKeys;
use nym_crypto::asymmetric::identity;

pub async fn get_identity_key(
    state: &tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Arc<identity::KeyPair>> {
    let config = {
        let state = state.read().await;
        state.load_config()?
    };

    let paths = config.storage_paths.common_paths.keys;

    // wtf, why are we loading EVERYTHING to just get identity key??
    let key_store = OnDiskKeys::from(paths);
    let key_manager =
        ClientKeys::load_keys(&key_store)
            .await
            .map_err(|err| BackendError::UnableToLoadKeys {
                source: Box::new(err),
            })?;
    let identity_keypair = key_manager.identity_keypair();

    Ok(identity_keypair)
}

fn key_filename<P: AsRef<Path>>(path: P) -> Result<String> {
    path.as_ref()
        .file_name()
        .ok_or(BackendError::CouldNotGetFilename)?
        .to_os_string()
        .into_string()
        .map_err(|_| BackendError::CouldNotGetFilename)
}

/// Export the gateway keys as a JSON string blob
#[tauri::command]
pub async fn export_keys(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<String> {
    let config = {
        let state = state.read().await;
        state.load_config()?
    };

    let key_paths = config.storage_paths.common_paths.keys;

    // Get key paths
    let ack_key_file = key_paths.ack_key();
    let gateway_shared_key_file = key_paths.gateway_shared_key();

    let pub_id_key_file = key_paths.public_identity_key();
    let priv_id_key_file = key_paths.private_identity_key();

    let pub_enc_key_file = key_paths.public_encryption_key();
    let priv_enc_key_file = key_paths.private_encryption_key();

    // Read file contents
    let ack_key = fs::read_to_string(ack_key_file)?;
    let gateway_shared_key = fs::read_to_string(gateway_shared_key_file)?;

    let pub_id_key = fs::read_to_string(pub_id_key_file)?;
    let priv_id_key = fs::read_to_string(priv_id_key_file)?;

    let pub_enc_key = fs::read_to_string(pub_enc_key_file)?;
    let priv_enc_key = fs::read_to_string(priv_enc_key_file)?;

    let ack_key_file = key_filename(&key_paths.ack_key_file)?;
    let gateway_shared_key_file = key_filename(&key_paths.gateway_shared_key_file)?;
    let pub_id_key_file = key_filename(&key_paths.public_identity_key_file)?;
    let priv_id_key_file = key_filename(&key_paths.private_identity_key_file)?;
    let pub_enc_key_file = key_filename(&key_paths.public_encryption_key_file)?;
    let priv_enc_key_file = key_filename(&key_paths.private_encryption_key_file)?;

    // Format and return as json
    let json = serde_json::json!({
        ack_key_file: ack_key,
        gateway_shared_key_file: gateway_shared_key,
        pub_id_key_file: pub_id_key,
        priv_id_key_file: priv_id_key,
        pub_enc_key_file: pub_enc_key,
        priv_enc_key_file: priv_enc_key,
    });

    Ok(serde_json::to_string_pretty(&json)?)
}
