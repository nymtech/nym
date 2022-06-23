// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::tx;
use cosmrs::tx::Gas;
use nym_types::currency::MajorCurrencyAmount;
use nym_types::fees::FeeDetails;
use validator_client::nymd::cosmwasm_client::types::{GasInfo, SimulateResponse};
use validator_client::nymd::{
    CosmosCoin, Fee, GasAdjustable, GasAdjustment, GasPrice, SigningNymdClient,
};
use validator_client::Client;

pub mod admin;
pub mod cosmos;
pub mod mixnet;
pub mod vesting;

pub(crate) fn detailed_fee(
    client: &Client<SigningNymdClient>,
    simulate_response: SimulateResponse,
) -> FeeDetails {
    let gas_price = client.nymd.gas_price().clone();
    let gas_adjustment = client.nymd.gas_adjustment();

    SimulateResult::new(simulate_response.gas_info, gas_price, gas_adjustment).detailed_fee()
}

// technically we could have also exposed a result: Option<AbciResult> field from the SimulateResponse,
// but in the context of the wallet it's really irrelevant and useless for the time being
pub(crate) struct SimulateResult {
    // As I mentioned somewhere before, from what I've seen in manual testing,
    // gas estimation does not exist if transaction itself fails to get executed.
    // for example if you attempt to send a 'BondMixnode' with invalid signature
    pub gas_info: Option<GasInfo>,
    pub gas_price: GasPrice,
    pub gas_adjustment: GasAdjustment,
}

impl SimulateResult {
    pub fn new(
        gas_info: Option<GasInfo>,
        gas_price: GasPrice,
        gas_adjustment: GasAdjustment,
    ) -> Self {
        SimulateResult {
            gas_info,
            gas_price,
            gas_adjustment,
        }
    }

    pub fn detailed_fee(&self) -> FeeDetails {
        let amount = self.to_fee_amount().map(MajorCurrencyAmount::from);
        FeeDetails {
            amount,
            fee: self.to_fee(),
        }
    }

    fn adjusted_gas(&self) -> Option<Gas> {
        self.gas_info
            .map(|gas_info| gas_info.gas_used.adjust_gas(self.gas_adjustment))
    }

    fn to_fee_amount(&self) -> Option<CosmosCoin> {
        self.adjusted_gas().map(|gas| &self.gas_price * gas)
    }

    fn to_fee(&self) -> Fee {
        self.adjusted_gas()
            .map(|gas| {
                let fee_amount = &self.gas_price * gas;
                tx::Fee::from_amount_and_gas(fee_amount, gas).into()
            })
            .unwrap_or_default()
    }
}
