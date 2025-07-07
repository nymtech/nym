// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::modules::auth::Auth;
use crate::cosmos_module::modules::authz::Authz;
use crate::cosmos_module::modules::bank::Bank;
use crate::cosmos_module::modules::capability::Capability;
use crate::cosmos_module::modules::consensus::Consensus;
use crate::cosmos_module::modules::crisis::Crisis;
use crate::cosmos_module::modules::distribution::Distribution;
use crate::cosmos_module::modules::evidence::Evidence;
use crate::cosmos_module::modules::feegrant::Feegrant;
use crate::cosmos_module::modules::gov_v1::GovV1;
use crate::cosmos_module::modules::gov_v1beta1::GovV1Beta1;
use crate::cosmos_module::modules::group::Group;
use crate::cosmos_module::modules::mint::Mint;
use crate::cosmos_module::modules::nft::Nft;
use crate::cosmos_module::modules::params::Params;
use crate::cosmos_module::modules::slashing::Slashing;
use crate::cosmos_module::modules::staking::Staking;
use crate::cosmos_module::modules::upgrade::Upgrade;
use crate::cosmos_module::modules::vesting::Vesting;
use crate::cosmos_module::modules::wasm::Wasm;
use crate::cosmos_module::CosmosModule;
use crate::error::ScraperError;
use cosmrs::proto::prost::Name;
use cosmrs::proto::traits::Message;
use cosmrs::Any;
use serde::Serialize;
use std::collections::HashMap;

pub(crate) fn default_proto_to_json<T: Message + Default + Serialize>(
    msg: &Any,
) -> Result<serde_json::Value, ScraperError> {
    let proto = <T as Message>::decode(msg.value.as_slice()).map_err(|error| {
        ScraperError::InvalidProtoRepresentation {
            type_url: msg.type_url.clone(),
            error,
        }
    })?;
    let mut base_serde =
        serde_json::to_value(&proto).map_err(|error| ScraperError::JsonSerialisationFailure {
            type_url: msg.type_url.clone(),
            error,
        })?;

    // in bdjuno's output we also had @type field with the type_url
    let obj = base_serde.as_object_mut().ok_or_else(|| {
        ScraperError::JsonSerialisationFailureNotObject {
            type_url: msg.type_url.clone(),
        }
    })?;
    obj.insert(
        "@type".to_string(),
        serde_json::Value::String(msg.type_url.clone()),
    );

    Ok(base_serde)
}

type ConvertFn = fn(&Any) -> Result<serde_json::Value, ScraperError>;

#[derive(Default)]
pub struct MessageRegistry {
    // type url to function converting bytes to proto and finally to json
    registered_types: HashMap<String, ConvertFn>,
}

impl MessageRegistry {
    pub fn new() -> Self {
        MessageRegistry {
            registered_types: Default::default(),
        }
    }

    pub fn register<T>(&mut self)
    where
        T: Message + Default + Name + Serialize + 'static,
    {
        self.register_with_custom_fn::<T>(default_proto_to_json::<T>)
    }

    #[allow(clippy::panic)]
    pub fn register_with_custom_fn<T>(&mut self, convert_fn: ConvertFn)
    where
        T: Message + Default + Name + Serialize + 'static,
    {
        if self
            .registered_types
            .insert(<T as Name>::type_url(), convert_fn)
            .is_some()
        {
            // don't allow duplicate registration because it most likely implies bug in the code
            panic!("duplicate registration of type {}", <T as Name>::type_url());
        }
    }

    pub fn try_decode(&self, raw: &Any) -> Result<serde_json::Value, ScraperError> {
        self.registered_types.get(&raw.type_url).ok_or(
            ScraperError::MissingTypeUrlRegistration {
                type_url: raw.type_url.clone(),
            },
        )?(raw)
    }
}

pub fn default_message_registry() -> MessageRegistry {
    let mut registry = MessageRegistry::new();
    let modules: Vec<Box<dyn CosmosModule>> = vec![
        Box::new(Auth),
        Box::new(Authz),
        Box::new(Bank),
        Box::new(Capability),
        Box::new(Consensus),
        Box::new(Wasm),
        Box::new(Crisis),
        Box::new(Distribution),
        Box::new(Evidence),
        Box::new(Feegrant),
        Box::new(GovV1),
        Box::new(GovV1Beta1),
        Box::new(Group),
        Box::new(Mint),
        Box::new(Nft),
        Box::new(Params),
        Box::new(Slashing),
        Box::new(Staking),
        Box::new(Upgrade),
        Box::new(Vesting),
    ];

    for module in modules {
        module.register_messages(&mut registry)
    }
    registry
}
