pub use coconut_rs::*;
use coconut_rs::{Attribute, Base58, BlindSignRequest, BlindedSignature, PublicKey};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

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

    pub fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes
            .iter()
            .map(|x| Attribute::try_from_bs58(x).unwrap())
            .collect()
    }

    pub fn verify(&self, validator_urls: Vec<String>) -> bool {
        let verification_key = get_aggregated_verification_key(validator_urls).unwrap();
        let params = Parameters::new(self.n_params).unwrap();
        coconut_rs::verify_credential(
            &params,
            &verification_key,
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
    public_key: PublicKey,
    public_attributes: Vec<String>,
    #[getset(get = "pub")]
    total_params: u32,
}

impl BlindSignRequestBody {
    pub fn new(
        blind_sign_request: &BlindSignRequest,
        public_key: &PublicKey,
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
    pub fn init() -> State {
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

fn get_verification_key(url: &str) -> Result<VerificationKey, String> {
    match attohttpc::get(format!("{}/v1/verification_key", url)).send() {
        Ok(resp) => {
            let verification_key_response: VerificationKeyResponse = resp.json().unwrap();
            Ok(verification_key_response.key)
        }
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn get_aggregated_verification_key(
    validator_urls: Vec<String>,
) -> Result<VerificationKey, String> {
    let mut verification_keys = Vec::new();
    let mut indices = Vec::new();

    for (idx, url) in validator_urls.iter().enumerate() {
        verification_keys.push(get_verification_key(url.as_ref())?);
        indices.push((idx + 1) as u64);
    }

    match aggregate_verification_keys(&verification_keys, Some(&indices)) {
        Ok(key) => Ok(key),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn get_aggregated_signature(
    validator_urls: Vec<String>,
    state: &State,
) -> Result<Signature, String> {
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
    Ok(aggregate_signature_shares(&signature_shares).unwrap())
}

pub fn prove_credential(
    idx: usize,
    validator_urls: Vec<String>,
    state: &State,
) -> Result<Theta, String> {
    let verification_key = coconut_interface::get_aggregated_verification_key(validator_urls)?;
    let signature = state
        .signatures
        .get(idx)
        .map_err(|e| "Got invalid signature idx")?;
    coconut_rs::prove_credential(
        &state.params,
        &verification_key,
        signature,
        &state.private_attributes,
    )
    .map_err(|e| format!("{:?}", e))
}
