pub use coconut_rs::*;
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};
use url::Url;
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

    pub async fn init(validator_urls: Vec<String>) -> Result<Self, String> {
        let mut state = State::init();
        let client = ValidatorAPIClient::new();
        let signature = get_aggregated_signature(validator_urls.clone(), &state, &client)
            .await
            .map_err(|e| format!("Could not aggregate signature from validators: {}", e))?;

        state.signatures.push(signature);
        let verification_key = get_aggregated_verification_key(validator_urls, &client).await?;
        let theta = prove_credential(0, &verification_key, &state)
            .await
            .map_err(|e| format!("Could not prove credential: {}", e))?;
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

async fn get_verification_key(
    url: &str,
    client: &ValidatorAPIClient,
) -> Result<VerificationKey, String> {
    let parsed_url =
        Url::parse(url).map_err(|e| format!("Could not parse validator url: {:?}", e))?;
    let verification_key_response: VerificationKeyResponse = client
        .query_validator_api(
            format!(
                "{}{}",
                VALIDATOR_API_CACHE_VERSION, VALIDATOR_API_VERIFICATION_KEY
            ),
            &parsed_url,
        )
        .await
        .map_err(|e| format!("Verification key could not be obtained: {:?}", e))?;
    Ok(verification_key_response.key)
}

pub async fn get_aggregated_verification_key(
    validator_urls: Vec<String>,
    client: &ValidatorAPIClient,
) -> Result<VerificationKey, String> {
    let mut verification_keys = Vec::new();
    let mut indices = Vec::new();

    for (idx, url) in validator_urls.iter().enumerate() {
        verification_keys.push(get_verification_key(url.as_ref(), client).await?);
        indices.push((idx + 1) as u64);
    }

    match aggregate_verification_keys(&verification_keys, Some(&indices)) {
        Ok(key) => Ok(key),
        Err(e) => Err(format!("{}", e)),
    }
}

pub async fn get_aggregated_signature(
    validator_urls: Vec<String>,
    state: &State,
    client: &ValidatorAPIClient,
) -> Result<Signature, String> {
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
        let parsed_url =
            Url::parse(url).map_err(|e| format!("Could not parse validator url: {:?}", e))?;
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
) -> Result<Theta, String> {
    let signature = state
        .signatures
        .get(idx)
        .ok_or("Got invalid signature idx")?;
    coconut_rs::prove_credential(
        &state.params,
        verification_key,
        signature,
        &state.private_attributes,
    )
    .map_err(|e| format!("{:?}", e))
}
