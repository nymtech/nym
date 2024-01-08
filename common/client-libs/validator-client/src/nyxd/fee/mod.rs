// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::Gas;
use crate::nyxd::{Coin, GasPrice};
use cosmrs::{tx, AccountId};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub mod gas_price;

pub type GasAdjustment = f32;

pub const DEFAULT_SIMULATED_GAS_MULTIPLIER: GasAdjustment = 1.5;

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
    Manual(tx::Fee),
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
    pub fn manual_with_gas_price(fee: Coin, gas_price: GasPrice) -> Self {
        let gas_limit = &fee / gas_price;

        Fee::Manual(tx::Fee::from_amount_and_gas(fee.into(), gas_limit))
    }

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
            let adjusted = (*self as f32 * adjustment).ceil();
            adjusted as u64
        }
    }
}
