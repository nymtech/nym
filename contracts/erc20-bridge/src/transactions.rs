// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::error::ContractError;
use crate::storage::{payments, status, Status};
use erc20_bridge_contract::payment::{LinkPaymentData, Payment};

pub(crate) fn link_payment(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    data: LinkPaymentData,
) -> Result<Response, ContractError> {
    let mut status_bucket = status(deps.storage);

    let verification_key = data.verification_key.as_bytes();
    let gateway_identity = data.gateway_identity.as_bytes();
    let message: Vec<u8> = verification_key
        .iter()
        .chain(gateway_identity.iter())
        .copied()
        .collect();
    let signature = data.signature.as_bytes();

    if let Ok(Some(_)) = status_bucket.may_load(&verification_key) {
        return Err(ContractError::PaymentAlreadyClaimed);
    }

    if !deps
        .api
        .ed25519_verify(&message, &signature, &verification_key)
        .map_err(|_| ContractError::ParseSignatureError)?
    {
        return Err(ContractError::BadSignature);
    }

    status_bucket.save(&verification_key, &Status::Unchecked)?;
    payments(deps.storage).save(
        &verification_key,
        &Payment::new(data.verification_key, data.gateway_identity, data.bandwidth),
    )?;

    Ok(Response::default())
}
