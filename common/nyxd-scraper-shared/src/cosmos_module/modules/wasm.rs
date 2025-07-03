// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::{default_proto_to_json, MessageRegistry};
use crate::cosmos_module::CosmosModule;
use crate::error::ScraperError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use cosmos_sdk_proto::cosmwasm::wasm::v1::{
    MsgAddCodeUploadParamsAddresses, MsgClearAdmin, MsgExecuteContract, MsgIbcCloseChannel,
    MsgIbcSend, MsgInstantiateContract, MsgInstantiateContract2, MsgMigrateContract, MsgPinCodes,
    MsgRemoveCodeUploadParamsAddresses, MsgStoreAndInstantiateContract, MsgStoreAndMigrateContract,
    MsgStoreCode, MsgSudoContract, MsgUnpinCodes, MsgUpdateAdmin, MsgUpdateContractLabel,
    MsgUpdateInstantiateConfig, MsgUpdateParams,
};
use cosmrs::Any;
use prost::Message;
use serde::Serialize;
use tracing::warn;

pub(crate) struct Wasm;

fn decode_wasm_message<T: Message + Default + Serialize>(
    msg: &Any,
) -> Result<serde_json::Value, ScraperError> {
    let field = "msg";
    // 1. perform basic decoding
    let mut base = default_proto_to_json::<T>(msg)?;
    let Some(encoded_field) = base.get_mut(field) else {
        warn!(
            "missing field 'msg' in wasm message of type {} - can't perform additional decoding",
            msg.type_url
        );
        return Ok(base);
    };

    // 2. decode 'msg' field
    let as_str =
        encoded_field
            .as_str()
            .ok_or(ScraperError::JsonWasmSerialisationFailureNotString {
                field: field.to_string(),
                type_url: msg.type_url.clone(),
            })?;

    let decoded = STANDARD.decode(as_str).map_err(|error| {
        ScraperError::JsonWasmSerialisationFailureInvalidBase64Encoding {
            field: field.to_string(),
            type_url: msg.type_url.clone(),
            error,
        }
    })?;

    // 3. replace original 'msg' with the new json
    let re_decoded: serde_json::Value = serde_json::from_slice(&decoded).map_err(|error| {
        ScraperError::JsonSerialisationFailure {
            type_url: format!("{}.{field}", msg.type_url),
            error,
        }
    })?;

    *encoded_field = re_decoded;
    Ok(base)
}

impl CosmosModule for Wasm {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgIbcSend>();
        registry.register::<MsgIbcCloseChannel>();
        registry.register::<MsgStoreCode>();

        registry.register_with_custom_fn::<MsgInstantiateContract>(|msg| {
            decode_wasm_message::<MsgInstantiateContract>(msg)
        });
        registry.register_with_custom_fn::<MsgInstantiateContract2>(|msg| {
            decode_wasm_message::<MsgInstantiateContract2>(msg)
        });
        registry.register_with_custom_fn::<MsgExecuteContract>(|msg| {
            decode_wasm_message::<MsgExecuteContract>(msg)
        });
        registry.register_with_custom_fn::<MsgMigrateContract>(|msg| {
            decode_wasm_message::<MsgMigrateContract>(msg)
        });
        registry.register_with_custom_fn::<MsgSudoContract>(|msg| {
            decode_wasm_message::<MsgSudoContract>(msg)
        });
        registry.register_with_custom_fn::<MsgStoreAndInstantiateContract>(|msg| {
            decode_wasm_message::<MsgStoreAndInstantiateContract>(msg)
        });
        registry.register_with_custom_fn::<MsgStoreAndMigrateContract>(|msg| {
            decode_wasm_message::<MsgStoreAndMigrateContract>(msg)
        });

        registry.register::<MsgUpdateAdmin>();
        registry.register::<MsgClearAdmin>();
        registry.register::<MsgUpdateInstantiateConfig>();
        registry.register::<MsgUpdateParams>();
        registry.register::<MsgPinCodes>();
        registry.register::<MsgUnpinCodes>();
        registry.register::<MsgAddCodeUploadParamsAddresses>();
        registry.register::<MsgRemoveCodeUploadParamsAddresses>();
        registry.register::<MsgUpdateContractLabel>();
    }
}
