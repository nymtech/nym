#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use coconut_interface::{
  self, get_aggregated_verification_key, Credential, Signature, State, ValidatorAPIClient,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tauri::command]
async fn randomise_credential(
  idx: usize,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
  {
    let mut state = state.write().await;
    let signature = state.signatures.remove(idx);
    let new = signature.randomise(&state.params);
    state.signatures.insert(idx, new);
  }
  {
    let state = state.read().await;
    Ok(state.signatures.clone())
  }
}

#[tauri::command]
async fn delete_credential(
  idx: usize,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
  {
    let mut state = state.write().await;
    let _ = state.signatures.remove(idx);
  }
  {
    let state = state.read().await;
    Ok(state.signatures.clone())
  }
}

#[tauri::command]
async fn list_credentials(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
  let state = state.read().await;
  Ok(state.signatures.clone())
}

#[tauri::command]
async fn verify_credential(
  idx: usize,
  validator_urls: Vec<String>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, String> {
  let state = state.read().await;
  let verification_key =
    get_aggregated_verification_key(validator_urls, &ValidatorAPIClient::default())
      .await
      .map_err(|e| format!("{:?}", e))?;
  let theta = coconut_interface::prove_credential(idx, &verification_key, &*state)
    .await
    .map_err(|e| format!("{:?}", e))?;

  let credential = Credential::new(
    state.n_attributes,
    &theta,
    &state.public_attributes,
    state
      .signatures
      .get(idx)
      .ok_or("Got invalid signature idx")?,
  );

  Ok(credential.verify(&verification_key).await)
}

#[tauri::command]
async fn get_credential(
  validator_urls: Vec<String>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
  let signature = {
    let state = state.read().await;
    coconut_interface::get_aggregated_signature(
      validator_urls,
      &*state,
      &ValidatorAPIClient::default(),
    )
    .await
    .map_err(|e| format!("{:?}", e))?
  };
  {
    let mut state = state.write().await;
    state.signatures.push(signature);
  }
  {
    let state = state.read().await;
    Ok(state.signatures.clone())
  }
}

fn main() {
  let public_attributes = vec![coconut_interface::hash_to_scalar("public_key")];
  let private_attributes = vec![coconut_interface::hash_to_scalar("private_key")];
  tauri::Builder::default()
    .manage(Arc::new(RwLock::new(
      State::init(public_attributes, private_attributes).unwrap(),
    )))
    .invoke_handler(tauri::generate_handler![
      get_credential,
      randomise_credential,
      delete_credential,
      list_credentials,
      verify_credential
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
