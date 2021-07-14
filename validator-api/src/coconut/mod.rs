use crate::Config;
use coconut_rs::{
    elgamal::PublicKey, Attribute, Base58, BlindSignRequest, BlindedSignature, Parameters,
};
use getset::{CopyGetters, Getters};
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};

#[derive(Getters, CopyGetters, Debug)]
pub struct InternalSignRequest {
    // Total number of paraeters to generate for
    #[getset(get_copy)]
    total_params: u32,
    #[getset(get)]
    public_attributes: Vec<Attribute>,
    #[getset(get)]
    public_key: PublicKey,
    #[getset(get)]
    blind_sign_request: BlindSignRequest,
}

impl InternalSignRequest {
    pub fn new(
        total_params: u32,
        public_attributes: Vec<Attribute>,
        public_key: PublicKey,
        blind_sign_request: BlindSignRequest,
    ) -> InternalSignRequest {
        InternalSignRequest {
            total_params,
            public_attributes,
            public_key,
            blind_sign_request,
        }
    }
}

pub fn blind_sign(request: InternalSignRequest, config: &Config) -> BlindedSignature {
    let params = Parameters::new(request.total_params()).unwrap();
    coconut_rs::blind_sign(
        &params,
        &config.keypair().secret_key(),
        request.public_key(),
        request.blind_sign_request(),
        request.public_attributes(),
    )
    .unwrap()
}

//  All strings are base58 encoded representations of structs
#[derive(Deserialize, CopyGetters)]
struct BlindSignRequestBody {
    blind_sign_request: String,
    public_key: String,
    public_attributes: Vec<String>,
    #[getset(get_copy)]
    total_params: u32,
}

#[derive(Serialize)]
struct BlindedSignatureResponse {
    blinded_signature: String,
}

#[post("/blind_sign", data = "<blind_sign_request_body>")]
//  Until we have serialization and deserialization traits we'll be using a crutch
async fn post_blind_sign(
    blind_sign_request_body: Json<BlindSignRequestBody>,
    config: &State<Config>,
) -> Json<BlindedSignatureResponse> {
    let blind_sign_request =
        BlindSignRequest::try_from_bs58(&blind_sign_request_body.blind_sign_request).unwrap();
    let public_key = PublicKey::try_from_bs58(&blind_sign_request_body.public_key).unwrap();
    let public_attributes: Vec<Attribute> = blind_sign_request_body
        .public_attributes
        .iter()
        .map(|x| Attribute::try_from_bs58(x).unwrap())
        .collect();
    let internal_request = InternalSignRequest::new(
        blind_sign_request_body.total_params(),
        public_attributes,
        public_key,
        blind_sign_request,
    );
    let blinded_signature = blind_sign(internal_request, config).to_bs58();
    Json(BlindedSignatureResponse { blinded_signature })
}
