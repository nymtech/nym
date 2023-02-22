use std::{ffi::OsStr, fs, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    error::{BackendError, Result},
    state::State,
};
use client_core::client::key_manager::KeyManager;
use client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use nym_crypto::asymmetric::identity;

pub async fn get_identity_key(
    state: &tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Arc<identity::KeyPair>> {
    let config = {
        let state = state.read().await;
        state.load_socks5_config()?
    };

    let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
    let key_manager = KeyManager::load_keys(&pathfinder)?;
    let identity_keypair = key_manager.identity_keypair();

    Ok(identity_keypair)
}

/// Export the gateway keys as a JSON string blob
#[tauri::command]
pub async fn export_keys(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<String> {
    let config = {
        let state = state.read().await;
        state.load_socks5_config()?
    };

    // Get key paths
    let ack_key_file = config.get_base().get_ack_key_file();
    let gateway_shared_key_file = config.get_base().get_gateway_shared_key_file();

    let pub_id_key_file = config.get_base().get_public_identity_key_file();
    let priv_id_key_file = config.get_base().get_private_identity_key_file();

    let pub_enc_key_file = config.get_base().get_public_encryption_key_file();
    let priv_enc_key_file = config.get_base().get_private_encryption_key_file();

    // Read file contents
    let ack_key = fs::read_to_string(ack_key_file.clone())?;
    let gateway_shared_key = fs::read_to_string(gateway_shared_key_file.clone())?;

    let pub_id_key = fs::read_to_string(pub_id_key_file.clone())?;
    let priv_id_key = fs::read_to_string(priv_id_key_file.clone())?;

    let pub_enc_key = fs::read_to_string(pub_enc_key_file.clone())?;
    let priv_enc_key = fs::read_to_string(priv_enc_key_file.clone())?;

    let ack_key_file = ack_key_file
        .file_name()
        .map(OsStr::to_string_lossy)
        .ok_or(BackendError::CouldNotGetFilename)?;
    let gateway_shared_key_file = gateway_shared_key_file
        .file_name()
        .map(OsStr::to_string_lossy)
        .ok_or(BackendError::CouldNotGetFilename)?;
    let pub_id_key_file = pub_id_key_file
        .file_name()
        .map(OsStr::to_string_lossy)
        .ok_or(BackendError::CouldNotGetFilename)?;
    let priv_id_key_file = priv_id_key_file
        .file_name()
        .map(OsStr::to_string_lossy)
        .ok_or(BackendError::CouldNotGetFilename)?;
    let pub_enc_key_file = pub_enc_key_file
        .file_name()
        .map(OsStr::to_string_lossy)
        .ok_or(BackendError::CouldNotGetFilename)?;
    let priv_enc_key_file = priv_enc_key_file
        .file_name()
        .map(OsStr::to_string_lossy)
        .ok_or(BackendError::CouldNotGetFilename)?;

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
