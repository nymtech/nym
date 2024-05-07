// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// use crate::error::ZkNymError;
// use crate::generic_scheme::get_params;
// use crate::types::{CredentialWrapper, ParametersWrapper, UnblindableShare};
// use crate::vpn_api_client::types::{
//     AttributesResponse, BandwidthVoucherResponse, PartialVerificationKeysResponse,
// };
// use nym_coconut::{
//     Base58, BlindSignRequest, BlindedSignature, PrivateAttribute, PublicAttribute, Scalar,
//     SignatureShare, VerificationKey,
// };
// use nym_credentials::coconut::bandwidth::issuance::Coin;
// use nym_credentials::coconut::bandwidth::issued::BandwidthCredentialIssuedDataVariant;
// use nym_credentials::coconut::bandwidth::voucher::BandwidthVoucherIssuedData;
// use nym_credentials::IssuedBandwidthCredential;
// use serde::{Deserialize, Serialize};
// use std::collections::HashMap;
// use tsify::Tsify;
// use wasm_bindgen::prelude::wasm_bindgen;
// use wasm_utils::console_error;
// use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};
//
// // tiny 'hacks' to just allow passing responses from vpn-api queries
// pub type NymIssuanceBandwidthVoucherOpts = AttributesResponse;
// pub type VoucherShares = BandwidthVoucherResponse;
// pub type VoucherIssuers = PartialVerificationKeysResponse;
//
// #[wasm_bindgen]
// #[derive(Debug)]
// #[allow(dead_code)]
// pub struct NymIssuanceBandwidthVoucher {
//     serial_number: PrivateAttribute,
//
//     binding_number: PrivateAttribute,
//
//     credential_amount: Coin,
//
//     prehashed_amount: PublicAttribute,
//     prehashed_type: PublicAttribute,
//
//     blind_sign_request: BlindSignRequest,
//     pedersen_commitments_openings: Zeroizing<Vec<Scalar>>,
// }
//
// #[wasm_bindgen]
// impl NymIssuanceBandwidthVoucher {
//     #[wasm_bindgen(js_name = "prepareNew", constructor)]
//     pub fn prepare_new(
//         opts: NymIssuanceBandwidthVoucherOpts,
//         parameters: Option<ParametersWrapper>,
//     ) -> Result<NymIssuanceBandwidthVoucher, ZkNymError> {
//         let deposit_amount: u128 = opts
//             .credential_amount_string
//             .parse()
//             .map_err(|_| ZkNymError::InvalidDepositAmount)?;
//         let credential_amount = Coin::new(deposit_amount, opts.credential_amount_denom);
//
//         let params = get_params(&parameters);
//         let serial_number = params.random_scalar();
//         let binding_number = params.random_scalar();
//
//         let prehashed_amount = Scalar::try_from_bs58(opts.bs58_prehashed_amount)?;
//         let prehashed_type = Scalar::try_from_bs58(opts.bs58_prehashed_type)?;
//
//         let public_attributes = vec![&prehashed_amount, &prehashed_type];
//         let private_attributes = vec![&serial_number, &binding_number];
//
//         let (pedersen_commitments_openings, blind_sign_request) =
//             nym_coconut::prepare_blind_sign(params, &private_attributes, &public_attributes)?;
//
//         Ok(NymIssuanceBandwidthVoucher {
//             serial_number,
//             binding_number,
//             credential_amount,
//             prehashed_amount,
//             prehashed_type,
//             blind_sign_request,
//             pedersen_commitments_openings: Zeroizing::new(pedersen_commitments_openings),
//         })
//     }
//
//     #[wasm_bindgen(js_name = "getBlindSignRequest")]
//     pub fn get_blind_sign_request(&self) -> String {
//         self.blind_sign_request.to_bs58()
//     }
//
//     #[wasm_bindgen(js_name = "unblindShare")]
//     pub fn unblind_share(&self, share: UnblindableShare) -> Result<CredentialWrapper, ZkNymError> {
//         let blinded_sig = BlindedSignature::try_from_bs58(share.blinded_share_bs58)?;
//         let vk = VerificationKey::try_from_bs58(share.issuer_key_bs58)?;
//
//         Ok(blinded_sig
//             .unblind(&vk, &self.pedersen_commitments_openings)
//             .into())
//     }
//
//     #[wasm_bindgen(js_name = "unblindShares")]
//     pub fn unblind_shares(
//         self,
//         shares: VoucherShares,
//         issuers: VoucherIssuers,
//     ) -> Result<NymIssuedBandwidthVoucher, ZkNymError> {
//         if shares.epoch_id != issuers.epoch_id {
//             console_error!(
//                 "the provided shares and issuers are not from the same epoch! {} and {}",
//                 shares.epoch_id,
//                 issuers.epoch_id
//             );
//             return Err(ZkNymError::InconsistentEpochId {
//                 shares: shares.epoch_id,
//                 issuers: issuers.epoch_id,
//             });
//         }
//
//         let mut decoded_keys = HashMap::new();
//         for key in issuers.keys {
//             let vk = VerificationKey::try_from_bs58(key.bs58_encoded_key)?;
//             decoded_keys.insert(key.node_index, vk);
//         }
//
//         let mut credential_shares = Vec::new();
//         for share in shares.shares {
//             let blinded_sig = BlindedSignature::try_from_bs58(share.bs58_encoded_share)?;
//             let Some(vk) = decoded_keys.get(&share.node_index) else {
//                 console_error!("received a share from issuer {} but did not receive a corresponding verification key!", share.node_index);
//                 continue;
//             };
//             let unblinded_sig = blinded_sig.unblind(vk, &self.pedersen_commitments_openings);
//             credential_shares.push(SignatureShare::new(unblinded_sig, share.node_index));
//         }
//
//         let signature = nym_coconut::aggregate_signature_shares(&credential_shares)?;
//
//         let voucher_data = BandwidthCredentialIssuedDataVariant::Voucher(
//             BandwidthVoucherIssuedData::new(self.credential_amount),
//         );
//
//         Ok(NymIssuedBandwidthVoucher {
//             inner: IssuedBandwidthCredential::new(
//                 self.serial_number,
//                 self.binding_number,
//                 signature,
//                 voucher_data,
//                 self.prehashed_type,
//                 shares.epoch_id,
//             ),
//         })
//     }
// }
//
// #[wasm_bindgen]
// pub struct NymIssuedBandwidthVoucher {
//     inner: IssuedBandwidthCredential,
// }
//
// #[wasm_bindgen]
// impl NymIssuedBandwidthVoucher {
//     #[wasm_bindgen(js_name = "ensureIsValid")]
//     pub fn ensure_is_valid(
//         &self,
//         master_vk: String,
//         parameters: Option<ParametersWrapper>,
//     ) -> bool {
//         let params = get_params(&parameters);
//         let Ok(master_vk) = VerificationKey::try_from_bs58(master_vk) else {
//             console_error!("malformed master verification key");
//             return false;
//         };
//
//         let spending_req = match self.inner.prepare_for_spending(&master_vk) {
//             Ok(req) => req,
//             Err(err) => {
//                 console_error!("failed to prepare spending request: {err}");
//                 return false;
//             }
//         };
//
//         spending_req.verify(params, &master_vk)
//     }
//
//     pub fn serialise(self) -> SerialisedNymIssuedBandwidthVoucher {
//         SerialisedNymIssuedBandwidthVoucher {
//             serialisation_revision:
//                 nym_credentials::coconut::bandwidth::issued::CURRENT_SERIALIZATION_REVISION,
//             bs58_encoded_data: bs58::encode(&self.inner.pack_v1()).into_string(),
//         }
//     }
// }
//
// #[derive(Tsify, Serialize, Deserialize, Debug, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
// #[tsify(into_wasm_abi, from_wasm_abi)]
// #[serde(rename_all = "camelCase")]
// pub struct SerialisedNymIssuedBandwidthVoucher {
//     pub serialisation_revision: u8,
//     pub bs58_encoded_data: String,
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::vpn_api_client::client::{new_client, NymVpnApiClient};
//
//     #[ignore]
//     #[tokio::test]
//     async fn end_to_end() -> anyhow::Result<()> {
//         let client = new_client("http://0.0.0.0:8080", "foomp")?;
//         let opts = client.get_prehashed_public_attributes().await?;
//         let issuance = NymIssuanceBandwidthVoucher::prepare_new(opts, None)?;
//
//         let shares = client
//             .get_bandwidth_voucher_blinded_shares(issuance.blind_sign_request.clone())
//             .await?;
//         let keys = client.get_partial_verification_keys().await?;
//         let master_key = client.get_master_verification_key().await?;
//
//         let voucher = issuance.unblind_shares(shares, keys)?;
//
//         println!(
//             "valid: {}",
//             voucher.ensure_is_valid(master_key.bs58_encoded_key, None)
//         );
//         let serialised = voucher.serialise();
//         println!("final: {serialised:#?}");
//
//         Ok(())
//     }
// }
