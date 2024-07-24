// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::tx;
use cosmrs::Gas;
use nym_types::fees::FeeDetails;
use nym_validator_client::nyxd::cosmwasm_client::types::GasInfo;
use nym_validator_client::nyxd::{CosmosCoin, Fee, GasAdjustable, GasAdjustment, GasPrice};

pub mod admin;
pub mod cosmos;
pub mod mixnet;
pub mod vesting;

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

    pub(crate) fn adjusted_gas(&self) -> Option<Gas> {
        self.gas_info
            .map(|gas_info| gas_info.gas_used.adjust_gas(self.gas_adjustment))
    }

    pub(crate) fn to_fee_amount(&self) -> Option<CosmosCoin> {
        self.adjusted_gas().map(|gas| &self.gas_price * gas)
    }

    pub(crate) fn to_fee(&self) -> Fee {
        self.adjusted_gas()
            .map(|gas| {
                let fee_amount = &self.gas_price * gas;
                tx::Fee::from_amount_and_gas(fee_amount, gas).into()
            })
            .unwrap_or_default()
    }
}
