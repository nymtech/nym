use crate::currency::{CurrencyDenom, DecCoin};
use nym_config::defaults::DenomDetails;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/Account.ts")
)]
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Account {
    pub client_address: String,
    pub base_mix_denom: String,

    // this should get refactored to just use a String, but for now it's fine as it reduces headache
    // for others
    pub display_mix_denom: CurrencyDenom,
}

impl Account {
    pub fn new(client_address: String, mix_denom: DenomDetails) -> Self {
        Account {
            client_address,
            base_mix_denom: mix_denom.base.to_owned(),
            display_mix_denom: mix_denom.display.parse().unwrap_or_default(),
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/AccountWithMnemonic.ts"
    )
)]
#[derive(Serialize, Deserialize)]
pub struct AccountWithMnemonic {
    pub account: Account,
    pub mnemonic: String,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/AccountEntry.ts")
)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountEntry {
    pub id: String,
    pub address: String,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/Balance.ts")
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
