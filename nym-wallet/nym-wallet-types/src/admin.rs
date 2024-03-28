// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Coin;
use nym_mixnet_contract_common::{
    ContractStateParams, OperatingCostRange as ContractOperatingCostRange,
    ProfitMarginRange as ContractProfitMarginRange,
};
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
    minimum_pledge: DecCoin,
    minimum_delegation: Option<DecCoin>,

    operating_cost: TauriOperatingCostRange,
    profit_margin: TauriProfitMarginRange,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/OperatingCostRange.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TauriOperatingCostRange {
    minimum: DecCoin,
    maximum: DecCoin,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/ProfitMarginRange.ts")
)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TauriProfitMarginRange {
    minimum: String,
    maximum: String,
}

impl TauriContractStateParams {
    pub fn from_mixnet_contract_contract_state_params(
        state_params: ContractStateParams,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        let rewarding_denom = &state_params.minimum_pledge.denom;
        let min_operating_cost_c = Coin {
            denom: rewarding_denom.into(),
            amount: state_params.interval_operating_cost.minimum,
        };
        let max_operating_cost_c = Coin {
            denom: rewarding_denom.into(),
            amount: state_params.interval_operating_cost.maximum,
        };

        Ok(TauriContractStateParams {
            minimum_pledge: reg
                .attempt_convert_to_display_dec_coin(state_params.minimum_pledge.into())?,
            minimum_delegation: state_params
                .minimum_delegation
                .map(|min_del| reg.attempt_convert_to_display_dec_coin(min_del.into()))
                .transpose()?,

            operating_cost: TauriOperatingCostRange {
                minimum: reg.attempt_convert_to_display_dec_coin(min_operating_cost_c.into())?,
                maximum: reg.attempt_convert_to_display_dec_coin(max_operating_cost_c.into())?,
            },
            profit_margin: TauriProfitMarginRange {
                minimum: state_params.profit_margin.minimum.to_string(),
                maximum: state_params.profit_margin.maximum.to_string(),
            },
        })
    }

    pub fn try_convert_to_mixnet_contract_params(
        self,
        reg: &RegisteredCoins,
    ) -> Result<ContractStateParams, TypesError> {
        assert_eq!(
            self.operating_cost.maximum.denom,
            self.operating_cost.minimum.denom
        );

        let min_operating_cost_c = reg.attempt_convert_to_base_coin(self.operating_cost.minimum)?;
        let max_operating_cost_c = reg.attempt_convert_to_base_coin(self.operating_cost.maximum)?;

        Ok(ContractStateParams {
            minimum_delegation: self
                .minimum_delegation
                .map(|min_del| reg.attempt_convert_to_base_coin(min_del))
                .transpose()?
                .map(Into::into),
            minimum_pledge: reg
                .attempt_convert_to_base_coin(self.minimum_pledge)?
                .into(),

            profit_margin: ContractProfitMarginRange {
                minimum: self.profit_margin.minimum.parse()?,
                maximum: self.profit_margin.maximum.parse()?,
            },
            interval_operating_cost: ContractOperatingCostRange {
                minimum: min_operating_cost_c.amount.into(),
                maximum: max_operating_cost_c.amount.into(),
            },
        })
    }
}
