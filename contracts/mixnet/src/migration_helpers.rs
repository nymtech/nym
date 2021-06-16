use cosmwasm_std::Storage;
use cosmwasm_std::{Coin, HumanAddr};
use cosmwasm_storage::{bucket, Bucket};
use mixnet_contract::{
    EncryptionStringPublicKeyWrapper, Gateway, GatewayBond, IdentityStringPublicKeyWrapper,
    MixNode, MixNodeBond,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::ops::Deref;

// this module will be changing per each contract migration as once migration is finished, it will be obsolete.

pub(crate) const PREFIX_MIXNODES_OLD: &[u8] = b"mixnodes";
pub(crate) const PREFIX_MIXNODES_OWNERS_OLD: &[u8] = b"mix-owners";
pub(crate) const PREFIX_GATEWAYS_OLD: &[u8] = b"gateways";
pub(crate) const PREFIX_GATEWAYS_OWNERS_OLD: &[u8] = b"gateway-owners";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
#[serde(rename = "MixNodeBond")]
pub(crate) struct MixNodeBondOld {
    pub amount: Vec<Coin>,
    pub owner: HumanAddr,
    pub mix_node: MixNodeOld,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
#[serde(rename = "MixNode")]
pub(crate) struct MixNodeOld {
    pub host: String,
    pub layer: u64,
    pub location: String,
    pub sphinx_key: String,
    pub identity_key: String,
    pub version: String,
}

impl TryInto<MixNodeBond> for MixNodeBondOld {
    // the actual error is irrelevant, we just want to know if it succeeded or not
    type Error = ();

    fn try_into(self) -> Result<MixNodeBond, Self::Error> {
        Ok(MixNodeBond {
            amount: self.amount,
            owner: self.owner,
            mix_node: MixNode {
                host: self.mix_node.host,
                layer: self.mix_node.layer,
                location: self.mix_node.location,
                sphinx_key: EncryptionStringPublicKeyWrapper(
                    <EncryptionStringPublicKeyWrapper as Deref>::Target::from_base58_string(
                        self.mix_node.sphinx_key,
                    )
                    .map_err(|_| ())?,
                ),
                identity_key: IdentityStringPublicKeyWrapper(
                    <IdentityStringPublicKeyWrapper as Deref>::Target::from_base58_string(
                        self.mix_node.identity_key,
                    )
                    .map_err(|_| ())?,
                ),
                version: self.mix_node.version,
            },
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
#[serde(rename = "GatewayBond")]
pub(crate) struct GatewayBondOld {
    pub amount: Vec<Coin>,
    pub owner: HumanAddr,
    pub gateway: GatewayOld,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
#[serde(rename = "Gateway")]
pub(crate) struct GatewayOld {
    pub mix_host: String,
    pub clients_host: String,
    pub location: String,
    pub sphinx_key: String,
    pub identity_key: String,
    pub version: String,
}

impl TryInto<GatewayBond> for GatewayBondOld {
    // the actual error is irrelevant, we just want to know if it succeeded or not
    type Error = ();

    fn try_into(self) -> Result<GatewayBond, Self::Error> {
        Ok(GatewayBond {
            amount: self.amount,
            owner: self.owner,
            gateway: Gateway {
                mix_host: self.gateway.mix_host,
                clients_host: self.gateway.clients_host,
                location: self.gateway.location,
                sphinx_key: EncryptionStringPublicKeyWrapper(
                    <EncryptionStringPublicKeyWrapper as Deref>::Target::from_base58_string(
                        self.gateway.sphinx_key,
                    )
                    .map_err(|_| ())?,
                ),
                identity_key: IdentityStringPublicKeyWrapper(
                    <IdentityStringPublicKeyWrapper as Deref>::Target::from_base58_string(
                        self.gateway.identity_key,
                    )
                    .map_err(|_| ())?,
                ),
                version: self.gateway.version,
            },
        })
    }
}

pub(crate) fn mixnodes_owners_old(storage: &mut dyn Storage) -> Bucket<HumanAddr> {
    bucket(storage, PREFIX_MIXNODES_OWNERS_OLD)
}

pub(crate) fn gateways_owners_old(storage: &mut dyn Storage) -> Bucket<HumanAddr> {
    bucket(storage, PREFIX_GATEWAYS_OWNERS_OLD)
}

pub(crate) fn mixnodes_old(storage: &mut dyn Storage) -> Bucket<MixNodeBondOld> {
    bucket(storage, PREFIX_MIXNODES_OLD)
}

pub(crate) fn gateways_old(storage: &mut dyn Storage) -> Bucket<GatewayBondOld> {
    bucket(storage, PREFIX_GATEWAYS_OLD)
}
