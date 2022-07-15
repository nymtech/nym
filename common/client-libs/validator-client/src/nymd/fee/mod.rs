// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::Coin;
use crate::nymd::Gas;
use cosmrs::{tx, AccountId};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub mod gas_price;

pub type GasAdjustment = f32;

pub const DEFAULT_SIMULATED_GAS_MULTIPLIER: GasAdjustment = 1.3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoFeeGrant {
    pub gas_adjustment: Option<GasAdjustment>,
    pub payer: Option<AccountId>,
    pub granter: Option<AccountId>,
}

impl Display for AutoFeeGrant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(gas_adjustment) = self.gas_adjustment {
            write!(f, "Feegrant in auto mode with {gas_adjustment} simulated multiplier with {:?} payer and {:?} granter", self.payer, self.granter)
        } else {
            write!(f, "Feegrant in auto mode with no custom simulated multiplier with {:?} payer and {:?} granter", self.payer, self.granter)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Fee {
    Manual(#[serde(with = "sealed::TxFee")] tx::Fee),
    Auto(Option<GasAdjustment>),
    PayerGranterAuto(AutoFeeGrant),
}

impl Display for Fee {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Fee::Manual(fee) => {
                write!(f, "Fee in manual mode with ")?;
                for fee in &fee.amount {
                    write!(f, "{}{} paid in fees, ", fee.amount, fee.denom)?;
                }
                write!(f, "{} set as gas limit, ", fee.gas_limit)?;
                if let Some(payer) = &fee.payer {
                    write!(f, "{payer} set as payer, ")?;
                }
                if let Some(granter) = &fee.granter {
                    write!(f, "{granter} set as granter")?;
                }
                Ok(())
            }
            Fee::Auto(Some(multiplier)) => {
                write!(f, "Fee in auto mode with {multiplier} simulated multiplier")
            }
            Fee::Auto(None) => write!(f, "Fee in auto mode with no custom simulated multiplier"),
            Fee::PayerGranterAuto(auto_feegrant) => write!(f, "{}", auto_feegrant),
        }
    }
}

impl Fee {
    pub fn new_payer_granter_auto(
        gas_adjustment: Option<GasAdjustment>,
        payer: Option<AccountId>,
        granter: Option<AccountId>,
    ) -> Self {
        Fee::PayerGranterAuto(AutoFeeGrant {
            gas_adjustment,
            payer,
            granter,
        })
    }
    pub fn try_get_manual_amount(&self) -> Option<Vec<Coin>> {
        match self {
            Fee::Manual(tx_fee) => Some(tx_fee.amount.iter().cloned().map(Into::into).collect()),
            _ => None,
        }
    }
}

impl From<tx::Fee> for Fee {
    fn from(fee: tx::Fee) -> Self {
        Fee::Manual(fee)
    }
}

impl From<GasAdjustment> for Fee {
    fn from(multiplier: GasAdjustment) -> Self {
        Fee::Auto(Some(multiplier))
    }
}

impl Default for Fee {
    fn default() -> Self {
        Fee::Auto(Some(DEFAULT_SIMULATED_GAS_MULTIPLIER))
    }
}

pub trait GasAdjustable {
    fn adjust_gas(&self, adjustment: GasAdjustment) -> Self;
}

impl GasAdjustable for Gas {
    fn adjust_gas(&self, adjustment: GasAdjustment) -> Self {
        if adjustment == 1.0 {
            *self
        } else {
            let adjusted = (self.value() as f32 * adjustment).ceil();
            (adjusted as u64).into()
        }
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
