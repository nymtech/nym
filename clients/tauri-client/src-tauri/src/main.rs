#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use coconut_interface::{self, Signature, State, Theta, ValidatorAPIClient};
use std::sync::Arc;
use tokio::sync::RwLock;

use thiserror::Error;

#[derive(Error, Debug)]
enum TauriClientError {
  #[error("Could not get {0} State, line {}!", line!())]
  State(&'static str),
}

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
    return Ok(state.signatures.clone());
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
async fn prove_credential(
  idx: usize,
  validator_urls: Vec<String>,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Theta, String> {
  let state = state.read().await;
  coconut_interface::prove_credential(idx, validator_urls, &*state, &ValidatorAPIClient::default())
    .await
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
    .await?
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
  tauri::Builder::default()
    .manage(Arc::new(RwLock::new(State::init())))
    .invoke_handler(tauri::generate_handler![
      get_credential,
      randomise_credential,
      delete_credential,
      list_credentials,
      prove_credential
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
