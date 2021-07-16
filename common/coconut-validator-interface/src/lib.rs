use coconut_rs::{Attribute, Base58, BlindSignRequest, BlindedSignature, PublicKey};
use serde::{Deserialize, Serialize};
pub use coconut_rs::*;

//  All strings are base58 encoded representations of structs
#[derive(Serialize, Deserialize, Debug)]
pub struct BlindSignRequestBody {
    blind_sign_request: String,
    public_key: String,
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
            blind_sign_request: blind_sign_request.to_bs58(),
            public_key: public_key.to_bs58(),
            public_attributes: public_attributes
                .iter()
                .map(|attr| attr.to_bs58())
                .collect(),
            total_params,
        }
    }

    pub fn blind_sign_request(&self) -> BlindSignRequest {
        BlindSignRequest::try_from_bs58(&self.blind_sign_request).unwrap()
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::try_from_bs58(&self.public_key).unwrap()
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
}

#[derive(Serialize, Deserialize)]
pub struct BlindedSignatureResponse {
    blinded_signature: String,
}

impl BlindedSignatureResponse {
    pub fn new(blinded_signature: BlindedSignature) -> BlindedSignatureResponse {
        BlindedSignatureResponse {
            blinded_signature: blinded_signature.to_bs58(),
        }
    }

    pub fn blinded_signature(&self) -> BlindedSignature {
        BlindedSignature::try_from_bs58(&self.blinded_signature).unwrap()
    }
}
