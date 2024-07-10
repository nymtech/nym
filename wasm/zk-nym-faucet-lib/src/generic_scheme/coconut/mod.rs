// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::coconut::types::{
    BlindSignRequestData, BlindSignRequestWrapper, BlindedCredentialWrapper,
    CredentialShareWrapper, CredentialWrapper, KeyPairWrapper, ParametersWrapper, ScalarsWrapper,
    VerificationKeyShareWrapper, VerificationKeyWrapper, VerifyCredentialRequestWrapper,
};
use crate::error::ZkNymError;
use crate::GLOBAL_COCONUT_PARAMS;
use nym_coconut::{hash_to_scalar, Parameters, SignerIndex};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tsify::Tsify;
use wasm_bindgen::prelude::wasm_bindgen;

pub mod types;

// works under the assumption of having 4 attributes in the underlying credential(s)
const DEFAULT_ATTRIBUTES: u32 = 4;

pub fn default_bandwidth_credential_params() -> &'static Parameters {
    static BANDWIDTH_CREDENTIAL_PARAMS: OnceLock<Parameters> = OnceLock::new();
    BANDWIDTH_CREDENTIAL_PARAMS.get_or_init(|| Parameters::new(DEFAULT_ATTRIBUTES).unwrap())
}

// attempt to extract appropriate system parameters in the following order:
// 1. attempt to get explicit provided value
// 2. then try a globally set value
// 3. finally fallback to sane default: the bandwidth credential params
pub(crate) fn get_params(explicit_params: &Option<ParametersWrapper>) -> &Parameters {
    if let Some(explicit) = explicit_params.as_ref() {
        return explicit;
    }

    if let Some(global) = GLOBAL_COCONUT_PARAMS.get() {
        return global;
    }

    default_bandwidth_credential_params()
}

#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize, Default)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct SetupOpts {
    #[tsify(optional)]
    pub num_attributes: Option<u32>,

    #[tsify(optional)]
    pub set_global: Option<bool>,
}

#[wasm_bindgen(js_name = "coconutSetup")]
pub fn setup(opts: SetupOpts) -> Result<ParametersWrapper, ZkNymError> {
    let num_attributes = opts.num_attributes.unwrap_or(DEFAULT_ATTRIBUTES);

    let params = nym_coconut::setup(num_attributes)?;

    if let Some(true) = opts.set_global {
        GLOBAL_COCONUT_PARAMS
            .set(params.clone())
            .map_err(|_| ZkNymError::GlobalParamsAlreadySet)?;
    }

    Ok(params.into())
}

#[wasm_bindgen(js_name = "coconutKeygen")]
pub fn keygen(parameters: Option<ParametersWrapper>) -> KeyPairWrapper {
    let params = get_params(&parameters);
    nym_coconut::keygen(params).into()
}

#[derive(Tsify, Debug, Clone, Copy, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct TtpKeygenOpts {
    pub threshold: u64,

    pub authorities: u64,
}

#[wasm_bindgen(js_name = "coconutTtpKeygen")]
pub fn ttp_keygen(
    opts: TtpKeygenOpts,
    parameters: Option<ParametersWrapper>,
) -> Result<Vec<KeyPairWrapper>, ZkNymError> {
    let params = get_params(&parameters);

    let keys = nym_coconut::ttp_keygen(params, opts.threshold, opts.authorities)?;
    Ok(keys.into_iter().map(Into::into).collect())
}

#[wasm_bindgen(js_name = "coconutSignSimple")]
pub fn sign_simple(
    attributes: Vec<String>,
    keys: &KeyPairWrapper,
    parameters: Option<ParametersWrapper>,
) -> Result<CredentialWrapper, ZkNymError> {
    let params = get_params(&parameters);

    let public_attributes = attributes
        .into_iter()
        .map(hash_to_scalar)
        .collect::<Vec<_>>();
    let attributes_ref = public_attributes.iter().collect::<Vec<_>>();

    nym_coconut::sign(params, keys.secret_key(), &attributes_ref)
        .map(Into::into)
        .map_err(Into::into)
}

#[wasm_bindgen(js_name = "coconutPrepareBlindSign")]
pub fn prepare_blind_sign(
    private_attributes: Vec<String>,
    public_attributes: Vec<String>,
    parameters: Option<ParametersWrapper>,
) -> Result<BlindSignRequestData, ZkNymError> {
    let params = get_params(&parameters);

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
    let (pedersen_commitments_openings, blind_sign_request) =
        nym_coconut::prepare_blind_sign(params, &private_attributes_ref, &public_attributes_ref)?;

    Ok(BlindSignRequestData {
        blind_sign_request,
        pedersen_commitments_openings,
    })
}

#[wasm_bindgen(js_name = "coconutBlindSign")]
pub fn blind_sign(
    keys: &KeyPairWrapper,
    blind_sign_request: &BlindSignRequestWrapper,
    public_attributes: Vec<String>,
    parameters: Option<ParametersWrapper>,
) -> Result<BlindedCredentialWrapper, ZkNymError> {
    let params = get_params(&parameters);

    let public_attributes = public_attributes
        .into_iter()
        .map(hash_to_scalar)
        .collect::<Vec<_>>();
    let public_attributes_ref = public_attributes.iter().collect::<Vec<_>>();

    nym_coconut::blind_sign(
        params,
        keys.secret_key(),
        blind_sign_request,
        &public_attributes_ref,
    )
    .map(Into::into)
    .map_err(Into::into)
}

#[wasm_bindgen(js_name = "coconutUnblindSignatureShare")]
pub fn unblind_signature_share(
    blinded_signature: &BlindedCredentialWrapper,
    partial_verification_key: &VerificationKeyWrapper,
    pedersen_commitments_openings: &ScalarsWrapper,
) -> CredentialWrapper {
    BlindedCredentialWrapper::unblind(
        blinded_signature,
        partial_verification_key,
        pedersen_commitments_openings,
    )
}

#[wasm_bindgen(js_name = "coconutUnblindAndVerifySignatureShare")]
pub fn unblind_and_verify_signature_share(
    blinded_signature: &BlindedCredentialWrapper,
    partial_verification_key: &VerificationKeyWrapper,
    request: &BlindSignRequestData,
    private_attributes: Vec<String>,
    public_attributes: Vec<String>,
    parameters: Option<ParametersWrapper>,
) -> Result<CredentialWrapper, ZkNymError> {
    BlindedCredentialWrapper::unblind_and_verify(
        blinded_signature,
        partial_verification_key,
        request,
        private_attributes,
        public_attributes,
        parameters,
    )
}

#[wasm_bindgen(js_name = "coconutAggregateSignatureShares")]
pub fn aggregate_signature_shares(
    shares: Vec<CredentialShareWrapper>,
) -> Result<CredentialWrapper, ZkNymError> {
    let shares = shares.into_iter().map(Into::into).collect::<Vec<_>>();

    nym_coconut::aggregate_signature_shares(&shares)
        .map(Into::into)
        .map_err(Into::into)
}

#[wasm_bindgen(js_name = "coconutAggregateSignatureSharesAndVerify")]
pub fn aggregate_signature_shares_and_verify(
    verification_key: &VerificationKeyWrapper,
    parameters: Option<ParametersWrapper>,
    private_attributes: Vec<String>,
    public_attributes: Vec<String>,
    shares: Vec<CredentialShareWrapper>,
) -> Result<CredentialWrapper, ZkNymError> {
    let params = get_params(&parameters);

    let public_attributes = public_attributes
        .into_iter()
        .map(hash_to_scalar)
        .collect::<Vec<_>>();

    let private_attributes = private_attributes
        .into_iter()
        .map(hash_to_scalar)
        .collect::<Vec<_>>();

    let attributes = private_attributes
        .iter()
        .chain(public_attributes.iter())
        .collect::<Vec<_>>();
    let shares = shares.into_iter().map(Into::into).collect::<Vec<_>>();

    nym_coconut::aggregate_signature_shares_and_verify(
        params,
        verification_key,
        &attributes,
        &shares,
    )
    .map(Into::into)
    .map_err(Into::into)
}

#[wasm_bindgen(js_name = "coconutAggregateVerificationKeyShares")]
pub fn aggregate_verification_key_shares(
    shares: Vec<VerificationKeyShareWrapper>,
) -> Result<VerificationKeyWrapper, ZkNymError> {
    let shares = shares.into_iter().map(Into::into).collect::<Vec<_>>();

    nym_coconut::aggregate_key_shares(&shares)
        .map(Into::into)
        .map_err(Into::into)
}

#[wasm_bindgen(js_name = "coconutAggregateVerificationKeys")]
pub fn aggregate_verification_keys(
    keys: Vec<VerificationKeyWrapper>,
    indices: Vec<SignerIndex>,
) -> Result<VerificationKeyWrapper, ZkNymError> {
    let keys = keys.into_iter().map(Into::into).collect::<Vec<_>>();

    nym_coconut::aggregate_verification_keys(&keys, Some(&indices))
        .map(Into::into)
        .map_err(Into::into)
}

#[wasm_bindgen(js_name = "coconutProveBandwidthCredential")]
pub fn prove_bandwidth_credential(
    verification_key: &VerificationKeyWrapper,
    credential: &CredentialWrapper,
    serial_number: String,
    binding_number: String,
    parameters: Option<ParametersWrapper>,
) -> Result<VerifyCredentialRequestWrapper, ZkNymError> {
    let params = get_params(&parameters);

    nym_coconut::prove_bandwidth_credential(
        params,
        verification_key,
        credential,
        &hash_to_scalar(serial_number),
        &hash_to_scalar(binding_number),
    )
    .map(Into::into)
    .map_err(Into::into)
}

#[wasm_bindgen(js_name = "coconutVerifyCredential")]
pub fn verify_credential(
    verification_key: &VerificationKeyWrapper,
    verification_request: &VerifyCredentialRequestWrapper,
    public_attributes: Vec<String>,
    parameters: Option<ParametersWrapper>,
) -> bool {
    let params = get_params(&parameters);

    let public_attributes = public_attributes
        .into_iter()
        .map(hash_to_scalar)
        .collect::<Vec<_>>();
    let attributes_ref = public_attributes.iter().collect::<Vec<_>>();

    nym_coconut::verify_credential(
        params,
        verification_key,
        verification_request,
        &attributes_ref,
    )
}

#[wasm_bindgen(js_name = "coconutVerifySimple")]
pub fn verify_simple(
    verification_key: &VerificationKeyWrapper,
    attributes: Vec<String>,
    credential: &CredentialWrapper,
    parameters: Option<ParametersWrapper>,
) -> bool {
    let params = get_params(&parameters);

    let public_attributes = attributes
        .into_iter()
        .map(hash_to_scalar)
        .collect::<Vec<_>>();
    let attributes_ref = public_attributes.iter().collect::<Vec<_>>();

    nym_coconut::verify(params, verification_key, &attributes_ref, credential)
}

#[wasm_bindgen(js_name = "coconutSimpleRandomiseCredential")]
pub fn simple_randomise_credential(
    credential: &CredentialWrapper,
    parameters: Option<ParametersWrapper>,
) -> CredentialWrapper {
    let params = get_params(&parameters);

    CredentialWrapper {
        inner: credential.inner.randomise_simple(params),
    }
}
