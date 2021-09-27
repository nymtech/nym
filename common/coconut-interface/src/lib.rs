// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

pub use coconut_rs::*;

#[derive(Serialize, Deserialize, Getters, CopyGetters, Clone)]
pub struct Credential {
    #[getset(get = "pub")]
    n_params: u32,
    #[getset(get = "pub")]
    theta: Theta,
    public_attributes: Vec<Vec<u8>>,
    #[getset(get = "pub")]
    signature: Signature,
}
impl Credential {
    pub fn new(
        n_params: u32,
        theta: Theta,
        public_attributes: Vec<Vec<u8>>,
        signature: &Signature,
    ) -> Credential {
        Credential {
            n_params,
            theta,
            public_attributes,
            signature: *signature,
        }
    }

    pub fn public_attributes(&self) -> Vec<Vec<u8>> {
        self.public_attributes.clone()
    }

    pub fn verify(&self, verification_key: &VerificationKey) -> bool {
        let params = Parameters::new(self.n_params).unwrap();
        let public_attributes = self
            .public_attributes
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<Attribute>>();
        coconut_rs::verify_credential(&params, verification_key, &self.theta, &public_attributes)
    }
}

#[derive(Serialize, Deserialize, Debug, Getters, CopyGetters)]
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
        blind_sign_request: BlindSignRequest,
        public_key: &coconut_rs::PublicKey,
        public_attributes: &[Attribute],
        total_params: u32,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            blind_sign_request,
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
