use crate::currency::DecCoin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Account.ts")
)]
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Account {
    pub client_address: String,
    pub mix_denom: String,
}

impl Account {
    pub fn new(client_address: String, mix_denom: String) -> Self {
        Account {
            client_address,
            mix_denom,
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/AccountWithMnemonic.ts")
)]
#[derive(Serialize, Deserialize)]
pub struct AccountWithMnemonic {
    pub account: Account,
    pub mnemonic: String,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/AccountEntry.ts")
)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountEntry {
    pub id: String,
    pub address: String,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Balance.ts")
)]
#[derive(Serialize, Deserialize)]
pub struct Balance {
    pub amount: DecCoin,
    pub printable_balance: String,
}

impl Balance {
    pub fn new(amount: DecCoin) -> Self {
        Balance {
            printable_balance: amount.to_string(),
            amount,
        }
    }
}
