use std::convert::TryFrom;

use cosmwasm_std::Uint128;
use serde::{Deserialize, Serialize};

use mixnet_contract_common::ContractStateParams;
use nym_types::error::TypesError;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/StateParams.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TauriContractStateParams {
    minimum_mixnode_pledge: String,
    minimum_gateway_pledge: String,
    mixnode_rewarded_set_size: u32,
    mixnode_active_set_size: u32,
}

impl From<ContractStateParams> for TauriContractStateParams {
    fn from(p: ContractStateParams) -> TauriContractStateParams {
        TauriContractStateParams {
            minimum_mixnode_pledge: p.minimum_mixnode_pledge.to_string(),
            minimum_gateway_pledge: p.minimum_gateway_pledge.to_string(),
            mixnode_rewarded_set_size: p.mixnode_rewarded_set_size,
            mixnode_active_set_size: p.mixnode_active_set_size,
        }
    }
}

impl TryFrom<TauriContractStateParams> for ContractStateParams {
    type Error = TypesError;

    fn try_from(p: TauriContractStateParams) -> Result<ContractStateParams, Self::Error> {
        Ok(ContractStateParams {
            minimum_mixnode_pledge: Uint128::try_from(p.minimum_mixnode_pledge.as_str())?,
            minimum_gateway_pledge: Uint128::try_from(p.minimum_gateway_pledge.as_str())?,
            mixnode_rewarded_set_size: p.mixnode_rewarded_set_size,
            mixnode_active_set_size: p.mixnode_active_set_size,
        })
    }
}
