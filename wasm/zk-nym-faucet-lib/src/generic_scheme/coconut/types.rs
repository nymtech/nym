// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ZkNymError;
use crate::{data_pointer_clone, wasm_wrapper, wasm_wrapper_bs58};
use nym_coconut::{
    hash_to_scalar, Base58, BlindSignRequest, BlindedSignature, KeyPair, Parameters, Scalar,
    SecretKey, Signature, SignatureShare, SignerIndex, VerificationKey, VerificationKeyShare,
    VerifyCredentialRequest,
};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::str::FromStr;
use tsify::Tsify;
use wasm_bindgen::prelude::wasm_bindgen;
use zeroize::{Zeroize, ZeroizeOnDrop};

wasm_wrapper!(Parameters, ParametersWrapper);
wasm_wrapper_bs58!(Signature, CredentialWrapper);
wasm_wrapper_bs58!(BlindedSignature, BlindedCredentialWrapper);
wasm_wrapper!(SignatureShare, CredentialShareWrapper);
wasm_wrapper_bs58!(Scalar, ScalarWrapper);

wasm_wrapper!(KeyPair, KeyPairWrapper);
wasm_wrapper!(SecretKey, SecretKeyWrapper);
wasm_wrapper!(BlindSignRequest, BlindSignRequestWrapper);
wasm_wrapper_bs58!(VerificationKey, VerificationKeyWrapper);
wasm_wrapper_bs58!(VerifyCredentialRequest, VerifyCredentialRequestWrapper);
wasm_wrapper!(VerificationKeyShare, VerificationKeyShareWrapper);

data_pointer_clone!(VerificationKeyShareWrapper);
data_pointer_clone!(CredentialShareWrapper);
data_pointer_clone!(BlindSignRequestWrapper);

#[wasm_bindgen]
impl BlindedCredentialWrapper {
    pub fn unblind(
        &self,
        partial_verification_key: &VerificationKeyWrapper,
        pedersen_commitments_openings: &ScalarsWrapper,
    ) -> CredentialWrapper {
        self.inner
            .unblind(partial_verification_key, pedersen_commitments_openings)
            .into()
    }

    #[wasm_bindgen(js_name = "unblindAndVerify")]
    pub fn unblind_and_verify(
        &self,
        partial_verification_key: &VerificationKeyWrapper,
        request: &BlindSignRequestData,
        private_attributes: Vec<String>,
        public_attributes: Vec<String>,
        parameters: Option<ParametersWrapper>,
    ) -> Result<CredentialWrapper, ZkNymError> {
        let params = super::get_params(&parameters);

        let public_attributes = public_attributes
            .into_iter()
            .map(hash_to_scalar)
            .collect::<Vec<_>>();
        let public_attributes_ref = public_attributes.iter().collect::<Vec<_>>();

        let private_attributes = private_attributes
            .into_iter()
            .map(hash_to_scalar)
            .collect::<Vec<_>>();
        let private_attributes_ref = private_attributes.iter().collect::<Vec<_>>();

        let unblinded_signature = self.inner.unblind_and_verify(
            params,
            partial_verification_key,
            &private_attributes_ref,
            &public_attributes_ref,
            &request.blind_sign_request.get_commitment_hash(),
            &request.pedersen_commitments_openings,
        )?;

        Ok(unblinded_signature.into())
    }
}

#[wasm_bindgen]
impl CredentialWrapper {
    #[wasm_bindgen(js_name = "intoShare")]
    pub fn into_share(self, index: SignerIndex) -> CredentialShareWrapper {
        CredentialShareWrapper {
            inner: SignatureShare::new(self.inner, index),
        }
    }
}

#[wasm_bindgen]
impl KeyPairWrapper {
    #[wasm_bindgen(js_name = "verificationKey")]
    pub fn verification_key(&self) -> VerificationKeyWrapper {
        self.inner.verification_key().clone().into()
    }

    pub fn index(&self) -> Option<SignerIndex> {
        self.inner.index
    }

    #[wasm_bindgen(js_name = "verificationKeyShare")]
    pub fn verification_key_share(&self) -> Option<VerificationKeyShareWrapper> {
        self.inner.to_verification_key_share().map(Into::into)
    }
}

#[wasm_bindgen]
pub struct BlindSignRequestData {
    pub(crate) blind_sign_request: BlindSignRequest,
    pub(crate) pedersen_commitments_openings: Vec<Scalar>,
}

#[wasm_bindgen]
impl BlindSignRequestData {
    #[wasm_bindgen(js_name = "blindSignRequest")]
    pub fn blind_sign_request(&self) -> BlindSignRequestWrapper {
        self.blind_sign_request.clone().into()
    }

    #[wasm_bindgen(js_name = "pedersenCommitmentsOpenings")]
    pub fn pedersen_commitments_openings(&self) -> ScalarsWrapper {
        ScalarsWrapper(self.pedersen_commitments_openings.clone())
    }
}

#[wasm_bindgen]
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ScalarsWrapper(pub(crate) Vec<Scalar>);

impl Deref for ScalarsWrapper {
    type Target = Vec<Scalar>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(
    Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Zeroize, ZeroizeOnDrop,
)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct KeypairWrapper {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Tsify, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct UnblindableShare {
    pub issuer_index: u64,
    pub issuer_key_bs58: String,
    pub blinded_share_bs58: String,
}

#[wasm_bindgen]
impl UnblindableShare {
    #[wasm_bindgen(constructor)]
    pub fn new(issuer_index: u64, issuer_key_bs58: String, blinded_share_bs58: String) -> Self {
        UnblindableShare {
            issuer_index,
            issuer_key_bs58,
            blinded_share_bs58,
        }
    }
}
