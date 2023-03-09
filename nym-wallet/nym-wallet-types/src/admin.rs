// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::ContractStateParams;
use nym_types::currency::{DecCoin, RegisteredCoins};
use nym_types::error::TypesError;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/StateParams.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TauriContractStateParams {
    minimum_mixnode_pledge: DecCoin,
    minimum_gateway_pledge: DecCoin,
    minimum_mixnode_delegation: Option<DecCoin>,
}

impl TauriContractStateParams {
    pub fn from_mixnet_contract_contract_state_params(
        state_params: ContractStateParams,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        Ok(TauriContractStateParams {
            minimum_mixnode_pledge: reg
                .attempt_convert_to_display_dec_coin(state_params.minimum_mixnode_pledge.into())?,
            minimum_gateway_pledge: reg
                .attempt_convert_to_display_dec_coin(state_params.minimum_gateway_pledge.into())?,
            minimum_mixnode_delegation: state_params
                .minimum_mixnode_delegation
                .map(|min_del| reg.attempt_convert_to_display_dec_coin(min_del.into()))
                .transpose()?,
        })
    }

    pub fn try_convert_to_mixnet_contract_params(
        self,
        reg: &RegisteredCoins,
    ) -> Result<ContractStateParams, TypesError> {
        Ok(ContractStateParams {
            minimum_mixnode_delegation: self
                .minimum_mixnode_delegation
                .map(|min_del| reg.attempt_convert_to_base_coin(min_del))
                .transpose()?
                .map(Into::into),
            minimum_mixnode_pledge: reg
                .attempt_convert_to_base_coin(self.minimum_mixnode_pledge)?
                .into(),
            minimum_gateway_pledge: reg
                .attempt_convert_to_base_coin(self.minimum_gateway_pledge)?
                .into(),
        })
    }
}
