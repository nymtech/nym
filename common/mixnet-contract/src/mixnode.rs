use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::fmt::Display;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use topology::asymmetric::{encryption, identity};
use topology::mix;
use topology::mix::MixnodeConversionError;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct MixNode {
    pub(crate) host: String,
    pub(crate) layer: u64,
    pub(crate) location: String,
    pub(crate) sphinx_key: String,
    /// Base58 encoded ed25519 EdDSA public key.
    pub(crate) identity_key: String,
    pub(crate) version: String,
}

impl MixNode {
    pub fn new(
        host: String,
        layer: u64,
        location: String,
        sphinx_key: String,
        identity_key: String,
        version: String,
    ) -> Self {
        MixNode {
            host,
            layer,
            location,
            sphinx_key,
            identity_key,
            version,
        }
    }

    fn resolve_hostname(&self) -> Result<SocketAddr, MixnodeConversionError> {
        self.host
            .to_socket_addrs()
            .map_err(MixnodeConversionError::InvalidAddress)?
            .next()
            .ok_or_else(|| {
                MixnodeConversionError::InvalidAddress(io::Error::new(
                    io::ErrorKind::Other,
                    "no valid socket address",
                ))
            })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct MixNodeBond {
    pub(crate) amount: Vec<Coin>,
    pub(crate) owner: HumanAddr,
    pub(crate) mix_node: MixNode,
}

impl MixNodeBond {
    pub fn new(amount: Vec<Coin>, owner: HumanAddr, mix_node: MixNode) -> Self {
        MixNodeBond {
            amount,
            owner,
            mix_node,
        }
    }

    pub fn amount(&self) -> &[Coin] {
        &self.amount
    }

    pub fn owner(&self) -> &HumanAddr {
        &self.owner
    }

    pub fn mix_node(&self) -> &MixNode {
        &self.mix_node
    }
}

impl Display for MixNodeBond {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "amount: {:?}, owner: {}", self.amount, self.owner)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct PagedResponse {
    pub nodes: Vec<MixNodeBond>,
    pub per_page: usize,
    pub start_next_after: Option<HumanAddr>,
}

impl PagedResponse {
    pub fn new(
        nodes: Vec<MixNodeBond>,
        per_page: usize,
        start_next_after: Option<HumanAddr>,
    ) -> Self {
        PagedResponse {
            nodes,
            per_page,
            start_next_after,
        }
    }
}

impl<'a> TryFrom<&'a MixNodeBond> for mix::Node {
    type Error = MixnodeConversionError;

    fn try_from(bond: &'a MixNodeBond) -> Result<Self, Self::Error> {
        Ok(mix::Node {
            location: bond.mix_node.location.clone(),
            host: bond.mix_node.resolve_hostname()?,
            identity_key: identity::PublicKey::from_base58_string(&bond.mix_node.identity_key)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&bond.mix_node.sphinx_key)?,
            layer: bond.mix_node.layer,
            version: bond.mix_node.version.clone(),
        })
    }
}

impl TryFrom<MixNodeBond> for mix::Node {
    type Error = MixnodeConversionError;

    fn try_from(bond: MixNodeBond) -> Result<Self, Self::Error> {
        mix::Node::try_from(&bond)
    }
}
