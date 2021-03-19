use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::Display;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use topology::asymmetric::{encryption, identity};
use topology::gateway;
use topology::gateway::GatewayConversionError;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct Gateway {
    pub(crate) mix_host: String,
    pub(crate) clients_host: String,
    pub(crate) location: String,
    pub(crate) sphinx_key: String,
    /// Base58 encoded ed25519 EdDSA public key of the gateway used to derive shared keys with clients
    pub(crate) identity_key: String,
    pub(crate) version: String,
}

impl Gateway {
    pub fn new(
        mix_host: String,
        clients_host: String,
        location: String,
        sphinx_key: String,
        identity_key: String,
        version: String,
    ) -> Self {
        Gateway {
            mix_host,
            clients_host,
            location,
            sphinx_key,
            identity_key,
            version,
        }
    }

    fn resolve_hostname(&self) -> Result<SocketAddr, GatewayConversionError> {
        self.mix_host
            .to_socket_addrs()
            .map_err(GatewayConversionError::InvalidAddress)?
            .next()
            .ok_or_else(|| {
                GatewayConversionError::InvalidAddress(io::Error::new(
                    io::ErrorKind::Other,
                    "no valid socket address",
                ))
            })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct GatewayBond {
    pub(crate) amount: Vec<Coin>,
    pub(crate) owner: HumanAddr,
    pub(crate) gateway: Gateway,
}

impl GatewayBond {
    pub fn new(amount: Vec<Coin>, owner: HumanAddr, gateway: Gateway) -> Self {
        GatewayBond {
            amount,
            owner,
            gateway,
        }
    }

    pub fn amount(&self) -> &[Coin] {
        &self.amount
    }

    pub fn owner(&self) -> &HumanAddr {
        &self.owner
    }

    pub fn gateway(&self) -> &Gateway {
        &self.gateway
    }
}

impl Display for GatewayBond {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.amount.len() != 1 {
            write!(f, "amount: {:?}, owner: {}", self.amount, self.owner)
        } else {
            write!(
                f,
                "amount: {} {}, owner: {}",
                self.amount[0].amount, self.amount[0].denom, self.owner
            )
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct PagedGatewayResponse {
    pub nodes: Vec<GatewayBond>,
    pub per_page: usize,
    pub start_next_after: Option<HumanAddr>,
}

impl PagedGatewayResponse {
    pub fn new(
        nodes: Vec<GatewayBond>,
        per_page: usize,
        start_next_after: Option<HumanAddr>,
    ) -> Self {
        PagedGatewayResponse {
            nodes,
            per_page,
            start_next_after,
        }
    }
}

impl<'a> TryFrom<&'a GatewayBond> for gateway::Node {
    type Error = GatewayConversionError;

    fn try_from(bond: &'a GatewayBond) -> Result<Self, Self::Error> {
        if bond.amount.len() > 1 {
            return Err(GatewayConversionError::InvalidStake);
        }
        Ok(gateway::Node {
            owner: bond.owner.0.clone(),
            stake: bond
                .amount
                .first()
                .map(|stake| stake.amount.into())
                .unwrap_or(0),
            location: bond.gateway.location.clone(),
            client_listener: bond.gateway.clients_host.clone(),
            mixnet_listener: bond.gateway.resolve_hostname()?,
            identity_key: identity::PublicKey::from_base58_string(&bond.gateway.identity_key)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&bond.gateway.sphinx_key)?,
            version: bond.gateway.version.clone(),
        })
    }
}

impl TryFrom<GatewayBond> for gateway::Node {
    type Error = GatewayConversionError;

    fn try_from(bond: GatewayBond) -> Result<Self, Self::Error> {
        gateway::Node::try_from(&bond)
    }
}
