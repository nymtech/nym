#![cfg_attr(
all(not(debug_assertions), target_os = "windows"),
windows_subsystem = "windows"
)]

use std::sync::Arc;

use tokio::sync::RwLock;
use url::Url;

use coconut_interface::{
  self, Attribute, Credential, hash_to_scalar, Parameters, Signature, Theta, VerificationKey,
};
use credentials::{obtain_aggregate_signature, obtain_aggregate_verification_key};

struct State {
    signatures: Vec<Signature>,
    n_attributes: u32,
    params: Parameters,
    serial_number: Attribute,
    binding_number: Attribute,
    voucher_value: Attribute,
    voucher_info: Attribute,
    aggregated_verification_key: Option<VerificationKey>,
}

impl State {
    fn init(public_attributes_bytes: Vec<Vec<u8>>, private_attributes_bytes: Vec<Vec<u8>>) -> State {
        let n_attributes = (public_attributes_bytes.len() + private_attributes_bytes.len()) as u32;
        let params = Parameters::new(n_attributes).unwrap();
        let public_attributes = public_attributes_bytes
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<Attribute>>();
        let private_attributes = private_attributes_bytes
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<Attribute>>();
        State {
            signatures: Vec::new(),
            n_attributes,
            params,
            serial_number: private_attributes[0],
            binding_number: private_attributes[1],
            voucher_value: public_attributes[0],
            voucher_info: public_attributes[1],
            aggregated_verification_key: None,
        }
    }
}

fn parse_url_validators(raw: &[String]) -> Result<Vec<Url>, String> {
    let mut parsed_urls = Vec::with_capacity(raw.len());
    for url in raw {
        let parsed_url: Url = url
            .parse()
            .map_err(|err| format!("one of validator urls is malformed - {}", err))?;
        parsed_urls.push(parsed_url)
    }
    Ok(parsed_urls)
}

#[tauri::command]
async fn randomise_credential(
    idx: usize,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
    let mut state = state.write().await;
    let signature = state.signatures.remove(idx);
    let (new_signature, _) = signature.randomise(&state.params);
    state.signatures.insert(idx, new_signature);
    Ok(state.signatures.clone())
}

#[tauri::command]
async fn delete_credential(
    idx: usize,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
    let mut state = state.write().await;
    state.signatures.remove(idx);
    Ok(state.signatures.clone())
}

#[tauri::command]
async fn list_credentials(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
    let state = state.read().await;
    Ok(state.signatures.clone())
}

async fn get_aggregated_verification_key(
    validator_urls: Vec<String>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<VerificationKey, String> {
    if let Some(verification_key) = &state.read().await.aggregated_verification_key {
        return Ok(verification_key.clone());
    }

    let parsed_urls = parse_url_validators(&validator_urls)?;
    let key = obtain_aggregate_verification_key(&parsed_urls)
        .await
        .map_err(|err| format!("failed to obtain aggregate verification key - {:?}", err))?;

    state
        .write()
        .await
        .aggregated_verification_key
        .replace(key.clone());

    Ok(key)
}

async fn prove_credential(
    idx: usize,
    validator_urls: Vec<String>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Theta, String> {
    let verification_key = get_aggregated_verification_key(validator_urls, state.clone()).await?;
    let state = state.read().await;

    if let Some(signature) = state.signatures.get(idx) {
        match coconut_interface::prove_bandwidth_credential(
            &state.params,
            &verification_key,
            signature,
            state.serial_number,
            state.binding_number,
        ) {
            Ok(theta) => Ok(theta),
            Err(e) => Err(format!("{:?}", e)),
        }
    } else {
        Err("Got invalid Signature idx".to_string())
    }
}

#[tauri::command]
async fn verify_credential(
    idx: usize,
    validator_urls: Vec<String>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, String> {
    // the API needs to be improved but at least it should compile (in theory)
    let verification_key =
        get_aggregated_verification_key(validator_urls.clone(), state.clone()).await?;
    let theta = prove_credential(idx, validator_urls, state.clone()).await?;

    let state = state.read().await;

    let public_attributes_bytes = vec![
        state.voucher_value.to_bytes().to_vec(),
        state.voucher_info.to_bytes().to_vec(),
    ];

    let credential = Credential::new(
        state.n_attributes,
        theta,
        public_attributes_bytes,
        state
            .signatures
            .get(idx)
            .ok_or("Got invalid signature idx")?,
    );

    Ok(credential.verify(&verification_key))
}

#[tauri::command]
async fn get_credential(
    validator_urls: Vec<String>,
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<Signature>, String> {
    let guard = state.read().await;
    let parsed_urls = parse_url_validators(&validator_urls)?;
    let public_attributes = vec![guard.voucher_value, guard.voucher_info];
    let private_attributes = vec![guard.serial_number, guard.binding_number];

    let signature = obtain_aggregate_signature(
        &guard.params,
        &public_attributes,
        &private_attributes,
        &parsed_urls,
    )
        .await
        .map_err(|err| format!("failed to obtain aggregate signature - {:?}", err))?;

    let mut state = state.write().await;
    state.signatures.push(signature);
    Ok(state.signatures.clone())
}

fn main() {
    let public_attributes = vec![b"public_key".to_vec()];
    let private_attributes = vec![b"private_key".to_vec()];
    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(State::init(
            public_attributes,
            private_attributes,
        ))))
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
