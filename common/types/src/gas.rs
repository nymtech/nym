use crate::currency::MajorCurrencyAmount;
use crate::error::TypesError;
use cosmrs::tx::Gas as CosmrsGas;
use serde::{Deserialize, Serialize};
use validator_client::nymd::cosmwasm_client::types::GasInfo as ValidatorClientGasInfo;
use validator_client::nymd::GasPrice;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Gas.ts")
)]
#[derive(Deserialize, Serialize, Clone)]
pub struct Gas {
    /// units of gas used
    pub gas_units: u64,
}

impl Gas {
    pub fn from_cosmrs_gas(value: CosmrsGas, _denom_minor: &str) -> Result<Gas, TypesError> {
        Ok(Gas {
            gas_units: value.value(),
        })

        // // TODO: use simulator struct to do conversion to fee
        // let value_u128 = Uint128::from(value.value());
        // let amount = Decimal::new(value_u128) * Decimal::from_str("0.0025")?;
        // Ok(Gas {
        //     gas_units: value.value(),
        //     amount: MajorCurrencyAmount::from_minor_decimal_and_denom(amount, denom_minor)?,
        // })
    }
    pub fn from_u64(value: u64, _denom_minor: &str) -> Result<Gas, TypesError> {
        Ok(Gas { gas_units: value })
        // todo!()
        // // TODO: use simulator struct to do conversion to fee
        // let value_u128 = Uint128::from(value);
        // let amount = Decimal::new(value_u128) * Decimal::from_str("0.0025")?;
        // Ok(Gas {
        //     gas_units: value,
        //     amount: MajorCurrencyAmount::from_minor_decimal_and_denom(amount, denom_minor)?,
        // })
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
        // terrible workaround, but I don't want to break the current flow (just yet)
        let gas_price = GasPrice::new_with_default_price(denom_minor)?;
        let fee = (&gas_price) * value.gas_used;
        Ok(GasInfo {
            gas_wanted: value.gas_wanted.value(),
            gas_used: value.gas_used.value(),
            fee: fee.into(),
        })
    }
    pub fn from_u64(
        gas_wanted: u64,
        gas_used: u64,
        denom_minor: &str,
    ) -> Result<GasInfo, TypesError> {
        // terrible workaround, but I don't want to break the current flow (just yet)
        let gas_price = GasPrice::new_with_default_price(denom_minor)?;
        let fee = (&gas_price) * CosmrsGas::from(gas_used);
        Ok(GasInfo {
            gas_wanted,
            gas_used,
            fee: fee.into(),
        })
    }
}
