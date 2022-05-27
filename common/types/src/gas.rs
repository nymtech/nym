use std::str::FromStr;

use cosmrs::tx::Gas as CosmrsGas;
use cosmwasm_std::{Decimal, Uint128};
use serde::{Deserialize, Serialize};

use validator_client::nymd::cosmwasm_client::types::GasInfo as ValidatorClientGasInfo;
use validator_client::nymd::GasPrice as ValidatorClientGasPrice;

use crate::currency::MajorCurrencyAmount;
use crate::error::TypesError;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Gas.ts")
)]
#[derive(Deserialize, Serialize, Clone)]
pub struct Gas {
    /// units of gas used
    pub gas_units: u64,

    /// gas units converted to fee as major coin amount
    pub amount: MajorCurrencyAmount,
}

impl Gas {
    pub fn from_cosmrs_gas(value: CosmrsGas, denom_minor: &str) -> Result<Gas, TypesError> {
        // TODO: use simulator struct to do conversion to fee
        let value_u128 = Uint128::from(value.value());
        let amount = Decimal::new(value_u128) * Decimal::from_str("0.0025")?;
        Ok(Gas {
            gas_units: value.value(),
            amount: MajorCurrencyAmount::from_minor_decimal_and_denom(amount, denom_minor)?,
        })
    }
    pub fn from_u64(value: u64, denom_minor: &str) -> Result<Gas, TypesError> {
        // TODO: use simulator struct to do conversion to fee
        let value_u128 = Uint128::from(value);
        let amount = Decimal::new(value_u128) * Decimal::from_str("0.0025")?;
        Ok(Gas {
            gas_units: value,
            amount: MajorCurrencyAmount::from_minor_decimal_and_denom(amount, denom_minor)?,
        })
    }
    pub fn from_gas_price(value: ValidatorClientGasPrice) -> Result<Gas, TypesError> {
        // TODO: use simulator struct to do conversion to fee
        let gas_units_str = (value.amount / Uint128::from_str("0.0025")?).to_string();
        let decimal_seperator_pos = gas_units_str.find('.').unwrap_or(gas_units_str.len());
        let gas_units = gas_units_str[..decimal_seperator_pos]
            .parse()
            .unwrap_or(0_u64);
        let ValidatorClientGasPrice { amount, denom } = value;
        Ok(Gas {
            gas_units,
            amount: MajorCurrencyAmount::from_minor_decimal_and_denom(amount, denom.as_ref())?,
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/GasInfo.ts")
)]
#[derive(Deserialize, Serialize)]
pub struct GasInfo {
    /// GasWanted is the maximum units of work we allow this tx to perform.
    pub gas_wanted: u64,

    /// GasUsed is the amount of gas actually consumed.
    pub gas_used: u64,

    /// gas units converted to fee as major coin amount
    pub fee: MajorCurrencyAmount,
}

impl GasInfo {
    pub fn from_validator_client_gas_info(
        value: ValidatorClientGasInfo,
        denom_minor: &str,
    ) -> Result<GasInfo, TypesError> {
        let fee = Gas::from_cosmrs_gas(value.gas_used, denom_minor)?.amount;
        Ok(GasInfo {
            gas_wanted: value.gas_wanted.value(),
            gas_used: value.gas_used.value(),
            fee,
        })
    }
    pub fn from_u64(
        gas_wanted: u64,
        gas_used: u64,
        denom_minor: &str,
    ) -> Result<GasInfo, TypesError> {
        let fee = Gas::from_u64(gas_used, denom_minor)?.amount;
        Ok(GasInfo {
            gas_wanted,
            gas_used,
            fee,
        })
    }
}
