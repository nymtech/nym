// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::error::ContractError;
use coconut_bandwidth_contract::deposit::DepositData;

pub(crate) fn deposit_funds(
    _deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    _data: DepositData,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::storage::payments_read;
//     use crate::support::tests::helpers;
//     use bandwidth_claim_contract::keys::PublicKey;
//     use cosmwasm_std::testing::{mock_env, mock_info};
//
//     #[test]
//     fn bad_signature_payment() {
//         let mut deps = helpers::init_contract();
//         let env = mock_env();
//         let info = mock_info("owner", &[]);
//
//         let payment_data = LinkPaymentData::new([1; 32], [2; 32], 42, [3; 64]);
//
//         assert_eq!(
//             link_payment(deps.as_mut(), env, info, payment_data),
//             Err(ContractError::BadSignature)
//         );
//     }
//
//     #[test]
//     fn good_payment() {
//         let mut deps = helpers::init_contract();
//         let env = mock_env();
//         let info = mock_info("owner", &[]);
//
//         let verification_key = [
//             78, 142, 213, 13, 39, 169, 76, 205, 242, 206, 129, 208, 190, 51, 139, 206, 245, 199,
//             120, 151, 181, 250, 192, 153, 123, 104, 129, 139, 60, 254, 243, 98,
//         ];
//         let gateway_identity = [
//             106, 76, 76, 238, 214, 177, 233, 112, 56, 33, 21, 201, 89, 42, 69, 196, 175, 56, 6,
//             110, 184, 167, 203, 63, 1, 167, 134, 102, 165, 215, 3, 212,
//         ];
//         let bandwidth = 42;
//         let signature = [
//             200, 134, 156, 198, 113, 180, 129, 90, 70, 28, 176, 201, 35, 208, 145, 28, 15, 16, 9,
//             110, 148, 188, 193, 75, 157, 201, 206, 211, 128, 215, 66, 207, 175, 155, 48, 24, 171,
//             254, 9, 37, 108, 205, 143, 37, 77, 189, 162, 52, 44, 130, 173, 60, 220, 22, 193, 3,
//             111, 90, 123, 147, 206, 8, 137, 1,
//         ];
//
//         let payment_data =
//             LinkPaymentData::new(verification_key, gateway_identity, bandwidth, signature);
//
//         assert!(link_payment(deps.as_mut(), env, info, payment_data).is_ok());
//
//         assert_eq!(
//             payments_read(&deps.storage)
//                 .load(&verification_key)
//                 .unwrap(),
//             Payment::new(
//                 PublicKey::new(verification_key),
//                 PublicKey::new(gateway_identity),
//                 bandwidth
//             )
//         );
//         assert_eq!(
//             status(&mut deps.storage).load(&verification_key).unwrap(),
//             Status::Unchecked
//         )
//     }
//
//     #[test]
//     fn double_spend_protection() {
//         let mut deps = helpers::init_contract();
//         let env = mock_env();
//         let info = mock_info("owner", &[]);
//
//         let verification_key = [
//             78, 142, 213, 13, 39, 169, 76, 205, 242, 206, 129, 208, 190, 51, 139, 206, 245, 199,
//             120, 151, 181, 250, 192, 153, 123, 104, 129, 139, 60, 254, 243, 98,
//         ];
//         let gateway_identity = [
//             106, 76, 76, 238, 214, 177, 233, 112, 56, 33, 21, 201, 89, 42, 69, 196, 175, 56, 6,
//             110, 184, 167, 203, 63, 1, 167, 134, 102, 165, 215, 3, 212,
//         ];
//         let bandwidth = 42;
//         let signature = [
//             200, 134, 156, 198, 113, 180, 129, 90, 70, 28, 176, 201, 35, 208, 145, 28, 15, 16, 9,
//             110, 148, 188, 193, 75, 157, 201, 206, 211, 128, 215, 66, 207, 175, 155, 48, 24, 171,
//             254, 9, 37, 108, 205, 143, 37, 77, 189, 162, 52, 44, 130, 173, 60, 220, 22, 193, 3,
//             111, 90, 123, 147, 206, 8, 137, 1,
//         ];
//
//         let payment_data =
//             LinkPaymentData::new(verification_key, gateway_identity, bandwidth, signature);
//
//         link_payment(deps.as_mut(), env.clone(), info.clone(), payment_data).unwrap();
//
//         // Only the verification key is used for double spending protection, the other data is irrelevant
//         let second_payment_data = LinkPaymentData::new(verification_key, [1; 32], 10, [2; 64]);
//         assert_eq!(
//             link_payment(deps.as_mut(), env, info, second_payment_data),
//             Err(ContractError::PaymentAlreadyClaimed)
//         )
//     }
// }
