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
    // TODO: discuss with @MS whether it makes sense:
    // 1. why are we restricting to single denom here? What if user holds both stake and mix currencies?
    // 2. what's the `contract_address`? is it mixnet? vesting? coconut? why does it relate to an account anyway?
    pub contract_address: String,
    pub client_address: String,
    pub denom: String,
}

impl Account {
    pub fn new(contract_address: String, client_address: String, denom: String) -> Self {
        Account {
            contract_address,
            client_address,
            denom,
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
