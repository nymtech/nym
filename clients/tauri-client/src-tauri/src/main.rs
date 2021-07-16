#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use coconut_rs::{
  aggregate_signature_shares, Base58, Parameters, Signature, SignatureShare,
};
use coconut_validator_interface::{BlindSignRequestBody, BlindedSignatureResponse};

const NUM_ATTRIBUTES: u32 = 3;

#[tauri::command]
fn randomise_credential(signature: String) -> String {
  let signature = Signature::try_from_bs58(signature).unwrap();
  let params = Parameters::new(NUM_ATTRIBUTES).unwrap();
  let new = signature.randomise(&params);
  new.to_bs58()
}

#[tauri::command]
fn get_credential(validator_urls: Vec<String>) -> String {
  let params = Parameters::new(NUM_ATTRIBUTES).unwrap();
  let public_attributes = params.n_random_scalars(2);
  let private_attributes = params.n_random_scalars(1);
  let elgamal_keypair = coconut_rs::elgamal_keygen(&params);

  let blind_sign_request = coconut_rs::prepare_blind_sign(
    &params,
    &elgamal_keypair.public_key(),
    &private_attributes,
    &public_attributes,
  )
  .unwrap();

  let blind_sign_request_body = BlindSignRequestBody::new(
    &blind_sign_request,
    elgamal_keypair.public_key(),
    &public_attributes,
    NUM_ATTRIBUTES,
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
      let blinded_signature = blinded_signature_response.blinded_signature();
      let unblinded_signature = blinded_signature.unblind(&elgamal_keypair.private_key());
      let signature_share = SignatureShare::new(unblinded_signature, (idx + 1) as u64);
      signature_shares.push(signature_share);
    }
  }

  let signature = aggregate_signature_shares(&signature_shares).unwrap();
  signature.to_bs58()
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      get_credential,
      randomise_credential
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
