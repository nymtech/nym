use crate::Config;
use coconut_rs::{elgamal::PublicKey, Attribute, BlindSignRequest, BlindedSignature, Parameters};
use coconut_validator_interface::{BlindSignRequestBody, BlindedSignatureResponse};
use getset::{CopyGetters, Getters};
use rocket::serde::json::Json;
use rocket::State;

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

#[post("/blind_sign", data = "<blind_sign_request_body>")]
//  Until we have serialization and deserialization traits we'll be using a crutch
pub async fn post_blind_sign(
    blind_sign_request_body: Json<BlindSignRequestBody>,
    config: &State<Config>,
) -> Json<BlindedSignatureResponse> {
    debug!("{:?}", blind_sign_request_body);
    let internal_request = InternalSignRequest::new(
        blind_sign_request_body.total_params(),
        blind_sign_request_body.public_attributes(),
        blind_sign_request_body.public_key(),
        blind_sign_request_body.blind_sign_request(),
    );
    let blinded_signature = blind_sign(internal_request, config);
    Json(BlindedSignatureResponse::new(blinded_signature))
}

// #[post("/verify_credential", data="<verify_credential_request_body>")]
// pub async fn post_verify_credential(
//     verify_credential_request_body: Json<VerifyCredentialRequestBody>,
//     condif: &State<Config>
// ) {

// }
