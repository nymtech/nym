pub use coconut_rs::*;
use coconut_rs::{Attribute, Base58, BlindSignRequest, BlindedSignature, PublicKey};
use getset::Getters;
use serde::{Deserialize, Serialize};

//  All strings are base58 encoded representations of structs
#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct BlindSignRequestBody {
    blind_sign_request: BlindSignRequest,
    public_key: PublicKey,
    public_attributes: Vec<String>,
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

    pub fn total_params(&self) -> u32 {
        self.total_params
    }

    pub fn blind_sign_request(&self) -> BlindSignRequest {
        self.blind_sign_request.clone()
    }

    pub fn public_key(&self) -> PublicKey {
        self.public_key.clone()
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
