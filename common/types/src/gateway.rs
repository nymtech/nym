use crate::currency::{DecCoin, RegisteredCoins};
use crate::error::TypesError;
use nym_mixnet_contract_common::{
    Gateway as MixnetContractGateway, GatewayBond as MixnetContractGatewayBond,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Gateway.ts")
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Serialize, JsonSchema)]
pub struct Gateway {
    pub host: String,
    pub mix_port: u16,
    pub clients_port: u16,
    pub location: String,
    pub sphinx_key: String,
    /// Base58 encoded ed25519 EdDSA public key of the gateway used to derive shared keys with clients
    pub identity_key: String,
    pub version: String,
}

impl From<MixnetContractGateway> for Gateway {
    fn from(value: MixnetContractGateway) -> Self {
        let MixnetContractGateway {
            host,
            mix_port,
            clients_port,
            location,
            sphinx_key,
            identity_key,
            version,
        } = value;

        Gateway {
            host,
            mix_port,
            clients_port,
            location,
            sphinx_key,
            identity_key,
            version,
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/GatewayBond.ts")
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, JsonSchema)]
pub struct GatewayBond {
    pub pledge_amount: DecCoin,
    pub owner: String,
    pub block_height: u64,
    pub gateway: Gateway,
    pub proxy: Option<String>,
}

impl GatewayBond {
    pub fn from_mixnet_contract_gateway_bond(
        bond: MixnetContractGatewayBond,
        reg: &RegisteredCoins,
    ) -> Result<GatewayBond, TypesError> {
        Ok(GatewayBond {
            pledge_amount: reg.attempt_convert_to_display_dec_coin(bond.pledge_amount.into())?,
            owner: bond.owner.to_string(),
            block_height: bond.block_height,
            gateway: bond.gateway.into(),
            proxy: bond.proxy.map(|p| p.into_string()),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayNodeDetailsResponse {
    pub identity_key: String,
    pub sphinx_key: String,
    pub bind_address: String,
    pub mix_port: u16,
    pub clients_port: u16,
    pub config_path: String,
    pub data_store: String,

    pub network_requester: Option<GatewayNetworkRequesterDetails>,
    pub ip_packet_router: Option<GatewayIpPacketRouterDetails>,
}

impl fmt::Display for GatewayNodeDetailsResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "config path: {}", self.config_path)?;
        writeln!(f, "identity key: {}", self.identity_key)?;
        writeln!(f, "sphinx key: {}", self.sphinx_key)?;
        writeln!(f, "bind address: {}", self.bind_address)?;
        writeln!(
            f,
            "mix port: {}, clients port: {}",
            self.mix_port, self.clients_port
        )?;

        writeln!(f, "data store is at: {}", self.data_store)?;

        if let Some(nr) = &self.network_requester {
            nr.fmt(f)?;
        }

        if let Some(ipr) = &self.ip_packet_router {
            ipr.fmt(f)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayNetworkRequesterDetails {
    pub enabled: bool,

    pub identity_key: String,
    pub encryption_key: String,

    pub open_proxy: bool,
    pub enabled_statistics: bool,

    // just a convenience wrapper around all the keys
    pub address: String,

    pub config_path: String,
}

impl fmt::Display for GatewayNetworkRequesterDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Network requester:")?;
        writeln!(f, "\tenabled: {}", self.enabled)?;
        writeln!(f, "\tconfig path: {}", self.config_path)?;

        writeln!(f, "\tidentity key: {}", self.identity_key)?;
        writeln!(f, "\tencryption key: {}", self.encryption_key)?;
        writeln!(f, "\taddress: {}", self.address)?;

        writeln!(f, "\tuses open proxy: {}", self.open_proxy)?;
        writeln!(f, "\tsends statistics: {}", self.enabled_statistics)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayIpPacketRouterDetails {
    pub enabled: bool,

    pub identity_key: String,
    pub encryption_key: String,

    // just a convenience wrapper around all the keys
    pub address: String,

    pub config_path: String,
}

impl fmt::Display for GatewayIpPacketRouterDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ip packet router:")?;
        writeln!(f, "\tenabled: {}", self.enabled)?;
        writeln!(f, "\tconfig path: {}", self.config_path)?;

        writeln!(f, "\tidentity key: {}", self.identity_key)?;
        writeln!(f, "\tencryption key: {}", self.encryption_key)?;
        writeln!(f, "\taddress: {}", self.address)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayWireguardDetails {
    pub enabled: bool,

    pub announced_port: u16,
    pub private_network_prefix: u8,
}

impl fmt::Display for GatewayWireguardDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "wireguard:")?;
        writeln!(f, "\tenabled: {}", self.enabled)?;

        writeln!(f, "\tannounced_port: {}", self.announced_port)?;
        writeln!(
            f,
            "\tprivate_network_prefix: {}",
            self.private_network_prefix
        )
    }
}
