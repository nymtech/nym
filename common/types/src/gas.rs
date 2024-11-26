use cosmrs::Gas as CosmrsGas;
use nym_validator_client::nyxd::cosmwasm_client::types::GasInfo as ValidatorClientGasInfo;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/Gas.ts")
)]
#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
pub struct Gas {
    /// units of gas used
    pub gas_units: u64,
}

impl Gas {
    pub fn from_u64(value: u64) -> Gas {
        Gas { gas_units: value }
    }
}

impl From<CosmrsGas> for Gas {
    fn from(gas: CosmrsGas) -> Self {
        Gas { gas_units: gas }
    }
}

impl From<i64> for Gas {
    fn from(value: i64) -> Self {
        Gas {
            gas_units: value.try_into().unwrap_or_default(),
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/GasInfo.ts")
)]
#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
pub struct GasInfo {
    /// GasWanted is the maximum units of work we allow this tx to perform.
    pub gas_wanted: Gas,

    /// GasUsed is the amount of gas actually consumed.
    pub gas_used: Gas,
}

impl From<ValidatorClientGasInfo> for GasInfo {
    fn from(info: ValidatorClientGasInfo) -> Self {
        GasInfo {
            gas_wanted: info.gas_wanted.into(),
            gas_used: info.gas_used.into(),
        }
    }
}

impl GasInfo {
    pub fn from_u64(gas_wanted: u64, gas_used: u64) -> GasInfo {
        GasInfo {
            gas_wanted: Gas::from_u64(gas_wanted),
            gas_used: Gas::from_u64(gas_used),
        }
    }
}
