#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use coconut_interface::{self, BlindSignRequestBody, BlindedSignatureResponse, State};
use coconut_rs::{
  aggregate_signature_shares, Attribute, Parameters, Signature, SignatureShare, Theta,
};
use std::sync::Arc;
use std::sync::RwLock;

use thiserror::Error;

#[derive(Error, Debug)]
enum TauriClientError {
  #[error("Could not get {0} State, line {}!", line!())]
  State(&'static str),
  #[error("Error getting data from validator API at {0}, line {}", line!())]
  ValidatorAPI(String),
}

#[tauri::command]
fn randomise_credential(
  idx: usize,
  state: tauri::State<Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
  match state.write() {
    Ok(mut state) => {
      let signature = state.signatures.remove(idx);
      let new = signature.randomise(&state.params);
      state.signatures.insert(idx, new);
    }
    Err(_e) => return Err(TauriClientError::State("write").to_string()),
  }

  match state.read() {
    Ok(state) => Ok(state.signatures.clone()),
    Err(_e) => Err(TauriClientError::State("read").to_string()),
  }
}

#[tauri::command]
fn delete_credential(
  idx: usize,
  state: tauri::State<Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
  match state.write() {
    Ok(mut state) => {
      let _ = state.signatures.remove(idx);
    }
    Err(_e) => return Err(TauriClientError::State("write").to_string()),
  }

  match state.read() {
    Ok(state) => Ok(state.signatures.clone()),
    Err(_e) => Err(TauriClientError::State("read").to_string()),
  }
}

#[tauri::command]
fn list_credentials(state: tauri::State<Arc<RwLock<State>>>) -> Result<Vec<Signature>, String> {
  match state.read() {
    Ok(state) => Ok(state.signatures.clone()),
    Err(_e) => Err(TauriClientError::State("read").to_string()),
  }
}

fn prove_credential(
  idx: usize,
  validator_urls: Vec<String>,
  state: tauri::State<Arc<RwLock<State>>>,
) -> Result<Theta, String> {
  match state.read() {
    Ok(state) => coconut_interface::prove_credential(idx, validator_urls, &*state),
    Err(_) => TauriClientError::State("read"),
  }
}

fn gateway_handshake(gateway_url: &str) {}

#[tauri::command]
fn get_credential(
  validator_urls: Vec<String>,
  state: tauri::State<Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
  let signature = match state.read() {
    Ok(state) => coconut_interface::get_aggregated_signature(validator_urls, &*state)?,
    Err(_e) => return Err(TauriClientError::State("read").to_string()),
  };
  match state.write() {
    Ok(mut state) => state.signatures.push(signature),
    Err(_e) => return Err(TauriClientError::State("write").to_string()),
  }
  match state.read() {
    Ok(state) => Ok(state.signatures.clone()),
    Err(_e) => Err(TauriClientError::State("read").to_string()),
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
      verify_credential
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
