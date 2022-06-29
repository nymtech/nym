use crate::currency::{DecCoin, RegisteredCoins};
use crate::error::TypesError;
use mixnet_contract_common::{
    MixNode as MixnetContractMixNode, MixNodeBond as MixnetContractMixNodeBond,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator_client::nymd::Coin;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Mixnode.ts")
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Serialize, JsonSchema)]
pub struct MixNode {
    pub host: String,
    pub mix_port: u16,
    pub verloc_port: u16,
    pub http_api_port: u16,
    pub sphinx_key: String,
    /// Base58 encoded ed25519 EdDSA public key.
    pub identity_key: String,
    pub version: String,
    pub profit_margin_percent: u8,
}

impl From<MixnetContractMixNode> for MixNode {
    fn from(value: MixnetContractMixNode) -> Self {
        let MixnetContractMixNode {
            host,
            mix_port,
            verloc_port,
            http_api_port,
            sphinx_key,
            identity_key,
            version,
            profit_margin_percent,
        } = value;

        Self {
            host,
            mix_port,
            verloc_port,
            http_api_port,
            sphinx_key,
            identity_key,
            version,
            profit_margin_percent,
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/MixNodeBond.ts")
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, JsonSchema)]
pub struct MixNodeBond {
    pub pledge_amount: DecCoin,
    pub total_delegation: DecCoin,
    pub owner: String,
    pub layer: String,
    pub block_height: u64,
    pub mix_node: MixNode,
    pub proxy: Option<String>,
    pub accumulated_rewards: Option<DecCoin>,
}

impl MixNodeBond {
    pub fn from_mixnet_contract_mixnode_bond(
        bond: MixnetContractMixNodeBond,
        reg: &RegisteredCoins,
    ) -> Result<MixNodeBond, TypesError> {
        let denom = bond.pledge_amount.denom.clone();
        Ok(MixNodeBond {
            pledge_amount: reg.attempt_convert_to_display_dec_coin(bond.pledge_amount.into())?,
            total_delegation: reg
                .attempt_convert_to_display_dec_coin(bond.total_delegation.into())?,
            owner: bond.owner.into_string(),
            layer: bond.layer.into(),
            bonding_height: block_height,
            mix_node: bond.mix_node.into(),
            proxy: bond.proxy.map(|p| p.to_string()),
            accumulated_rewards: bond
                .accumulated_rewards
                .map(|reward| {
                    // here we're making an assumption that rewards always use the same denom as the pledge
                    // (which I think is a reasonable assumption)
                    reg.attempt_convert_to_display_dec_coin(Coin::new(reward.u128(), denom))
                })
                .transpose()?,
        })
    }
}
