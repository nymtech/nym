// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//use cosmrs::Decimal;
//use cosmrs::Denom as CosmosDenom;
//use cosmrs::Coin as CosmosCoin;
//use cosmwasm_std::Coin as CosmWasmCoin;
//use cosmwasm_std::Uint128;

use cosmwasm_std::Uint128;

#[derive(Debug)]
pub struct Coin {
    pub denom: String,
    pub amount: Uint128,
}

impl From<cosmrs::Coin> for Coin {
    fn from(_: cosmrs::Coin) -> Self {
        todo!()
    }
}

impl From<cosmwasm_std::Coin> for Coin {
    fn from(_: cosmwasm_std::Coin) -> Self {
        todo!()
    }
}

