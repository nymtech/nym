// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::tx;
use serde::{Deserialize, Serialize};

pub mod gas_price;
pub mod helpers;

pub const DEFAULT_SIMULATED_GAS_MULTIPLIER: f32 = 1.3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Fee {
    Manual(#[serde(with = "sealed::TxFee")] tx::Fee),
    Auto(Option<f32>),
}

impl From<tx::Fee> for Fee {
    fn from(fee: tx::Fee) -> Self {
        Fee::Manual(fee)
    }
}

impl From<f32> for Fee {
    fn from(multiplier: f32) -> Self {
        Fee::Auto(Some(multiplier))
    }
}

impl Default for Fee {
    fn default() -> Self {
        Fee::Auto(Some(DEFAULT_SIMULATED_GAS_MULTIPLIER))
    }
}

// a workaround to provide serde implementation for tx::Fee. We don't want to ever expose any of those
// types to the public and ideally they will get replaced by proper implementation inside comrs
mod sealed {
    use cosmrs::tx::{self, Gas};
    use cosmrs::Coin as CosmosCoin;
    use cosmrs::{AccountId, Decimal as CosmosDecimal, Denom as CosmosDenom};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    fn cosmos_denom_inner_getter(val: &CosmosDenom) -> String {
        val.as_ref().to_string()
    }

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "CosmosDenom")]
    struct Denom(#[serde(getter = "cosmos_denom_inner_getter")] String);

    impl From<Denom> for CosmosDenom {
        fn from(val: Denom) -> Self {
            val.0.parse().unwrap()
        }
    }

    fn cosmos_decimal_inner_getter(val: &CosmosDecimal) -> u64 {
        // haha, this code is so disgusting. I'll make a PR on cosmrs to slightly alleviate those issues...
        // note: unwrap here is fine as the to_string is just returning a stringified u64 which, well, is a valid u64
        val.to_string().parse().unwrap()
    }

    // at the time of writing it the current cosmrs' Decimal is extremely limited...
    #[derive(Serialize, Deserialize)]
    #[serde(remote = "CosmosDecimal")]
    struct Decimal(#[serde(getter = "cosmos_decimal_inner_getter")] u64);

    impl From<Decimal> for CosmosDecimal {
        fn from(val: Decimal) -> Self {
            val.0.into()
        }
    }

    #[derive(Serialize, Deserialize, Clone)]
    struct Coin {
        #[serde(with = "Denom")]
        denom: CosmosDenom,
        #[serde(with = "Decimal")]
        amount: CosmosDecimal,
    }

    impl From<Coin> for CosmosCoin {
        fn from(val: Coin) -> Self {
            CosmosCoin {
                denom: val.denom,
                amount: val.amount,
            }
        }
    }

    impl From<CosmosCoin> for Coin {
        fn from(val: CosmosCoin) -> Self {
            Coin {
                denom: val.denom,
                amount: val.amount,
            }
        }
    }

    fn coin_vec_ser<S: Serializer>(val: &[CosmosCoin], serializer: S) -> Result<S::Ok, S::Error> {
        let vec: Vec<Coin> = val.iter().cloned().map(Into::into).collect();
        vec.serialize(serializer)
    }
    fn coin_vec_deser<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<CosmosCoin>, D::Error> {
        let vec: Vec<Coin> = Deserialize::deserialize(deserializer)?;
        Ok(vec.iter().cloned().map(Into::into).collect())
    }

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "tx::Fee")]
    pub(super) struct TxFee {
        #[serde(serialize_with = "coin_vec_ser")]
        #[serde(deserialize_with = "coin_vec_deser")]
        pub amount: Vec<CosmosCoin>,
        pub gas_limit: Gas,
        pub payer: Option<AccountId>,
        pub granter: Option<AccountId>,
    }
}
