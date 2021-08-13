pub mod error;

use digest::Digest;
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};
use sha2::digest::generic_array::typenum::Unsigned;
use sha2::Sha256;
use std::convert::TryFrom;
use url::Url;

use crate::error::CoconutInterfaceError;
pub use coconut_rs::*;
pub use validator_client::validator_api::Client as ValidatorAPIClient;
use validator_client::validator_api::{
    error::ValidatorAPIClientError, VALIDATOR_API_BLIND_SIGN, VALIDATOR_API_CACHE_VERSION,
    VALIDATOR_API_VERIFICATION_KEY,
};

#[derive(Serialize, Deserialize, Getters, CopyGetters, Clone)]
pub struct Credential {
    #[getset(get = "pub")]
    n_params: u32,
    #[getset(get = "pub")]
    theta: Theta,
    public_attributes: Vec<String>,
    #[getset(get = "pub")]
    signature: Signature,
}
impl Credential {
    pub fn new(
        n_params: u32,
        theta: &Theta,
        public_attributes: &[Attribute],
        signature: &Signature,
    ) -> Credential {
        Credential {
            n_params,
            theta: theta.clone(),
            public_attributes: public_attributes
                .iter()
                .map(|attr| attr.to_bs58())
                .collect(),
            signature: *signature,
        }
    }

    pub async fn init(
        validator_urls: Vec<String>,
        public_key: crypto::asymmetric::identity::PublicKey,
    ) -> Result<Self, CoconutInterfaceError> {
        let public_attributes = vec![hash_to_scalar(public_key.to_bytes())];
        let private_attributes = vec![hash_to_scalar("Bandwidth: infinite (for now)")];
        let mut state = State::init(public_attributes, private_attributes)?;
        let client = ValidatorAPIClient::new();
        let signature = get_aggregated_signature(validator_urls.clone(), &state, &client).await?;

        state.signatures.push(signature);
        let verification_key = get_aggregated_verification_key(validator_urls, &client).await?;
        let theta = prove_credential(0, &verification_key, &state).await?;
        Ok(Credential::new(
            state.n_attributes,
            &theta,
            &*state.public_attributes,
            &signature,
        ))
    }

    pub fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes
            .iter()
            .map(|x| Attribute::try_from_bs58(x).unwrap())
            .collect()
    }

    pub async fn verify(&self, verification_key: &VerificationKey) -> bool {
        let params = Parameters::new(self.n_params).unwrap();
        coconut_rs::verify_credential(
            &params,
            verification_key,
            &self.theta,
            &self.public_attributes(),
        )
    }
}

#[derive(Getters, CopyGetters)]
pub struct VerifyCredentialBody {
    #[getset(get = "pub")]
    n_params: u32,
    #[getset(get = "pub")]
    theta: Theta,
    public_attributes: Vec<String>,
}

impl VerifyCredentialBody {
    pub fn new(
        n_params: u32,
        theta: &Theta,
        public_attributes: &[Attribute],
    ) -> VerifyCredentialBody {
        VerifyCredentialBody {
            n_params,
            theta: theta.clone(),
            public_attributes: public_attributes
                .iter()
                .map(|attr| attr.to_bs58())
                .collect(),
        }
    }

    pub fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes
            .iter()
            .map(|x| Attribute::try_from_bs58(x).unwrap())
            .collect()
    }
}
//  All strings are base58 encoded representations of structs
#[derive(Serialize, Deserialize, Debug, Getters, CopyGetters)]
pub struct BlindSignRequestBody {
    #[getset(get = "pub")]
    blind_sign_request: BlindSignRequest,
    #[getset(get = "pub")]
    public_key: coconut_rs::PublicKey,
    public_attributes: Vec<String>,
    #[getset(get = "pub")]
    total_params: u32,
}

impl BlindSignRequestBody {
    pub fn new(
        blind_sign_request: &BlindSignRequest,
        public_key: &coconut_rs::PublicKey,
        public_attributes: &[Attribute],
        total_params: u32,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            blind_sign_request: blind_sign_request.clone(),
            public_key: public_key.clone(),
            public_attributes: public_attributes
                .iter()
                .map(|attr| attr.to_bs58())
                .collect(),
            total_params,
        }
    }

    pub fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes
            .iter()
            .map(|x| Attribute::try_from_bs58(x).unwrap())
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlindedSignatureResponse {
    pub blinded_signature: BlindedSignature,
}

impl BlindedSignatureResponse {
    pub fn new(blinded_signature: BlindedSignature) -> BlindedSignatureResponse {
        BlindedSignatureResponse { blinded_signature }
    }
}

#[derive(Serialize, Deserialize)]
pub struct VerificationKeyResponse {
    pub key: VerificationKey,
}

impl VerificationKeyResponse {
    pub fn new(key: VerificationKey) -> VerificationKeyResponse {
        VerificationKeyResponse { key }
    }
}

pub struct State {
    pub signatures: Vec<Signature>,
    pub n_attributes: u32,
    pub params: Parameters,
    pub public_attributes: Vec<Attribute>,
    pub private_attributes: Vec<Attribute>,
}

impl State {
    pub fn init(
        public_attributes: Vec<Attribute>,
        private_attributes: Vec<Attribute>,
    ) -> Result<State, CoconutInterfaceError> {
        let n_attributes_usize = public_attributes.len() + private_attributes.len();
        let n_attributes = u32::try_from(n_attributes_usize).map_err(|_| {
            CoconutInterfaceError::TooManyTotalAttributes(n_attributes_usize, u32::MAX)
        })?;
        let params = Parameters::new(n_attributes).unwrap();
        Ok(State {
            signatures: Vec::new(),
            n_attributes,
            params,
            public_attributes,
            private_attributes,
        })
    }
}

async fn get_verification_key(
    url: &str,
    client: &ValidatorAPIClient,
) -> Result<VerificationKey, CoconutInterfaceError> {
    let parsed_url = Url::parse(url).map_err(CoconutInterfaceError::from)?;
    let verification_key_response: VerificationKeyResponse = client
        .query_validator_api(
            format!(
                "{}{}",
                VALIDATOR_API_CACHE_VERSION, VALIDATOR_API_VERIFICATION_KEY
            ),
            &parsed_url,
        )
        .await
        .map_err(CoconutInterfaceError::from)?;
    Ok(verification_key_response.key)
}

pub async fn get_aggregated_verification_key(
    validator_urls: Vec<String>,
    client: &ValidatorAPIClient,
) -> Result<VerificationKey, CoconutInterfaceError> {
    let mut verification_keys = Vec::new();
    let mut indices = Vec::new();

    for (idx, url) in validator_urls.iter().enumerate() {
        verification_keys.push(get_verification_key(url.as_ref(), client).await?);
        indices.push((idx + 1) as u64);
    }

    aggregate_verification_keys(&verification_keys, Some(&indices))
        .map_err(CoconutInterfaceError::AggregateVerificationKeyError)
}

pub async fn get_aggregated_signature(
    validator_urls: Vec<String>,
    state: &State,
    client: &ValidatorAPIClient,
) -> Result<Signature, CoconutInterfaceError> {
    let elgamal_keypair = coconut_rs::elgamal_keygen(&state.params);
    let blind_sign_request = coconut_rs::prepare_blind_sign(
        &state.params,
        elgamal_keypair.public_key(),
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
        let parsed_url = Url::parse(url).map_err(CoconutInterfaceError::from)?;
        let response: Result<BlindedSignatureResponse, ValidatorAPIClientError> = client
            .post_validator_api(
                format!(
                    "{}{}",
                    VALIDATOR_API_CACHE_VERSION, VALIDATOR_API_BLIND_SIGN
                ),
                &blind_sign_request_body,
                &parsed_url,
            )
            .await;
        if let Ok(blinded_signature_response) = response {
            let blinded_signature = blinded_signature_response.blinded_signature;
            let unblinded_signature = blinded_signature.unblind(elgamal_keypair.private_key());
            let signature_share = SignatureShare::new(unblinded_signature, (idx + 1) as u64);
            signature_shares.push(signature_share);
        }
    }
    Ok(aggregate_signature_shares(&signature_shares).unwrap())
}

pub async fn prove_credential(
    idx: usize,
    verification_key: &VerificationKey,
    state: &State,
) -> Result<Theta, CoconutInterfaceError> {
    let signature = state
        .signatures
        .get(idx)
        .ok_or(CoconutInterfaceError::InvalidSignatureIdx(idx))?;
    coconut_rs::prove_credential(
        &state.params,
        verification_key,
        signature,
        &state.private_attributes,
    )
    .map_err(CoconutInterfaceError::ProveCredentialError)
}

pub fn hash_to_scalar<M>(msg: M) -> Attribute
where
    M: AsRef<[u8]>,
{
    let mut h = Sha256::new();
    h.update(msg);
    let digest = h.finalize();

    let mut bytes = [0u8; 64];
    let pad_size = 64usize
        .checked_sub(<Sha256 as Digest>::OutputSize::to_usize())
        .unwrap_or_default();

    bytes[pad_size..].copy_from_slice(&digest);

    Attribute::from_bytes_wide(&bytes)
}
