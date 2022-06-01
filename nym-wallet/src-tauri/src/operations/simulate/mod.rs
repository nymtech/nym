// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_types::fees::FeeDetails;
use validator_client::nymd::cosmwasm_client::types::GasInfo;
use validator_client::nymd::{tx, CosmosCoin, Fee, GasPrice};

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
}

impl SimulateResult {
    pub fn new(gas_info: Option<GasInfo>, gas_price: GasPrice) -> Self {
        SimulateResult {
            gas_info,
            gas_price,
        }
    }

    pub(crate) fn to_fee_amount(&self) -> Option<CosmosCoin> {
        self.gas_info
            .map(|gas_info| &self.gas_price * gas_info.gas_used)
    }

    pub(crate) fn to_fee(&self) -> Fee {
        self.to_fee_amount()
            .and_then(|fee_amount| {
                self.gas_info.map(|gas_info| {
                    let gas_limit = gas_info.gas_used;
                    tx::Fee::from_amount_and_gas(fee_amount, gas_limit).into()
                })
            })
            .unwrap_or_default()
    }
}
