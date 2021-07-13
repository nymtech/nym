use crate::Config;
use coconut_rs::{elgamal::PublicKey, Attribute, BlindSignRequest, BlindedSignature, Parameters};
use getset::{CopyGetters, Getters};

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
