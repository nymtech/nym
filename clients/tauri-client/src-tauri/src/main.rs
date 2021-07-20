#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use coconut_rs::{aggregate_signature_shares, Attribute, Parameters, Signature, SignatureShare};
use coconut_rs::{aggregate_verification_keys, VerificationKey};
use coconut_validator_interface::{
  BlindSignRequestBody, BlindedSignatureResponse, VerificationKeyResponse,
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

struct State {
  signatures: Vec<Signature>,
  n_attributes: u32,
  params: Parameters,
  public_attributes: Vec<Attribute>,
  private_attributes: Vec<Attribute>,
}

impl State {
  fn init() -> State {
    let n_attributes: u32 = 3;
    let params = Parameters::new(n_attributes).unwrap();
    let public_attributes = params.n_random_scalars(2);
    let private_attributes = params.n_random_scalars(1);
    State {
      signatures: Vec::new(),
      n_attributes,
      params,
      public_attributes,
      private_attributes,
    }
  }
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

fn get_verification_key(url: &str) -> Result<VerificationKey, String> {
  match attohttpc::get(format!("{}/v1/verification_key", url)).send() {
    Ok(resp) => {
      let verification_key_response: VerificationKeyResponse = resp.json().unwrap();
      Ok(verification_key_response.key)
    }
    Err(_e) => Err(TauriClientError::ValidatorAPI(url.to_string()).to_string()),
  }
}

#[tauri::command]
fn verify_credential(
  idx: usize,
  validator_urls: Vec<String>,
  state: tauri::State<Arc<RwLock<State>>>,
) -> Result<String, String> {
  let mut verification_keys = Vec::new();
  let mut indices = Vec::new();

  for (idx, url) in validator_urls.iter().enumerate() {
    verification_keys.push(get_verification_key(url)?);
    indices.push((idx + 1) as u64);
  }

  let verification_key = aggregate_verification_keys(&verification_keys, Some(&indices)).unwrap();

  match state.read() {
    Ok(state) => {
      if let Some(signature) = state.signatures.get(idx) {
        let theta = coconut_rs::prove_credential(
          &state.params,
          &verification_key,
          signature,
          &state.private_attributes,
        )
        .unwrap();
        assert!(coconut_rs::verify_credential(
          &state.params,
          &verification_key,
          &theta,
          &state.public_attributes
        ));
      }
    }
    Err(_e) => return Err(TauriClientError::State("read").to_string()),
  }

  Ok("Success!".to_string())
}

#[tauri::command]
fn get_credential(
  validator_urls: Vec<String>,
  state: tauri::State<Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
  let signature_shares = match state.read() {
    Ok(state) => {
      let elgamal_keypair = coconut_rs::elgamal_keygen(&state.params);
      let blind_sign_request = coconut_rs::prepare_blind_sign(
        &state.params,
        &elgamal_keypair.public_key(),
        &state.private_attributes,
        &state.public_attributes,
      )
      .unwrap();
      let blind_sign_request_body = BlindSignRequestBody::new(
        &blind_sign_request,
        elgamal_keypair.public_key(),
        &state.public_attributes,
        state.n_attributes,
      );

      let mut signature_shares = vec![];

      for (idx, url) in validator_urls.iter().enumerate() {
        let resp = attohttpc::post(format!("{}/v1/blind_sign", url))
          .json(&blind_sign_request_body)
          .unwrap()
          .send()
          .unwrap();

        if resp.is_success() {
          let blinded_signature_response: BlindedSignatureResponse = resp.json().unwrap();
          let blinded_signature = blinded_signature_response.blinded_signature;
          let unblinded_signature = blinded_signature.unblind(&elgamal_keypair.private_key());
          let signature_share = SignatureShare::new(unblinded_signature, (idx + 1) as u64);
          signature_shares.push(signature_share);
        }
      }
      signature_shares
    }
    Err(_e) => return Err(TauriClientError::State("read").to_string()),
  };

  let signature = aggregate_signature_shares(&signature_shares).unwrap();
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
