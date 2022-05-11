use crate::currency::MajorCurrencyAmount;
use crate::error::TypesError;
use mixnet_contract_common::{
    Gateway as MixnetContractGateway, GatewayBond as MixnetContractGatewayBond,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Gateway.ts")
)]
#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize, JsonSchema)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct GatewayBond {
    pub pledge_amount: MajorCurrencyAmount,
    pub owner: String,
    pub block_height: u64,
    pub gateway: Gateway,
    pub proxy: Option<String>,
}

impl GatewayBond {
    pub fn from_mixnet_contract_gateway_bond(
        bond: Option<MixnetContractGatewayBond>,
    ) -> Result<Option<GatewayBond>, TypesError> {
        match bond {
            Some(bond) => {
                let bond: GatewayBond = bond.try_into()?;
                Ok(Some(bond))
            }
            None => Ok(None),
        }
    }
}

impl TryFrom<MixnetContractGatewayBond> for GatewayBond {
    type Error = TypesError;

    fn try_from(value: MixnetContractGatewayBond) -> Result<Self, Self::Error> {
        let MixnetContractGatewayBond {
            pledge_amount,
            owner,
            block_height,
            gateway,
            proxy,
        } = value;

        let pledge_amount: MajorCurrencyAmount = pledge_amount.try_into()?;

        Ok(GatewayBond {
            pledge_amount,
            owner: owner.into_string(),
            block_height,
            gateway: gateway.into(),
            proxy: proxy.map(|p| p.into_string()),
        })
    }
}
