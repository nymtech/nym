use coconut_interface::{
    elgamal::PublicKey, Attribute, BlindSignRequest, BlindSignRequestBody, BlindedSignature,
    BlindedSignatureResponse, KeyPair, Parameters, VerificationKeyResponse,
};
use getset::{CopyGetters, Getters};
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use rocket::State;
use validator_client::validator_api::VALIDATOR_API_CACHE_VERSION;

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

    pub fn stage(key_pair: KeyPair) -> AdHoc {
        AdHoc::on_ignite("Internal Sign Request Stage", |rocket| async {
            rocket.manage(key_pair).mount(
                VALIDATOR_API_CACHE_VERSION,
                routes![post_blind_sign, get_verification_key],
            )
        })
    }
}

pub fn blind_sign(request: InternalSignRequest, key_pair: &KeyPair) -> BlindedSignature {
    let params = Parameters::new(request.total_params()).unwrap();
    coconut_interface::blind_sign(
        &params,
        &key_pair.secret_key(),
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
    key_pair: &State<KeyPair>,
) -> Json<BlindedSignatureResponse> {
    debug!("{:?}", blind_sign_request_body);
    let internal_request = InternalSignRequest::new(
        *blind_sign_request_body.total_params(),
        blind_sign_request_body.public_attributes(),
        blind_sign_request_body.public_key().clone(),
        blind_sign_request_body.blind_sign_request().clone(),
    );
    let blinded_signature = blind_sign(internal_request, key_pair);
    Json(BlindedSignatureResponse::new(blinded_signature))
}

#[get("/verification_key")]
pub async fn get_verification_key(key_pair: &State<KeyPair>) -> Json<VerificationKeyResponse> {
    Json(VerificationKeyResponse::new(key_pair.verification_key()))
}
