use crate::currency::DecCoin;
use nym_validator_client::nyxd::Fee;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/FeeDetails.ts")
)]
pub struct FeeDetails {
    // expected to be used by the wallet in order to display detailed fee information to the user
    pub amount: Option<DecCoin>,
    #[cfg_attr(feature = "generate-ts", ts(as = "ts_type_helpers::Fee"))]
    pub fee: Fee,
}

impl FeeDetails {
    pub fn new(amount: Option<DecCoin>, fee: Fee) -> Self {
        FeeDetails { amount, fee }
    }
}

// this should really be sealed and NEVER EVER used as "normal" types,
// but due to our typescript requirements, we have to expose it to generate
// the types...
#[cfg(feature = "generate-ts")]
pub mod ts_type_helpers {
    use nym_validator_client::nyxd::GasAdjustment;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
    #[ts(export_to = "ts-packages/types/src/types/rust/Fee.ts")]
    pub enum Fee {
        Manual(CosmosFee),
        Auto(Option<GasAdjustment>),
    }

    #[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
    #[ts(export_to = "ts-packages/types/src/types/rust/CosmosFee.ts")]
    // this should corresponds to cosmrs::tx::Fee
    // IMPORTANT NOTE: this should work as of cosmrs 0.7.1 due to their `FromStr` implementations
    // on the type. The below struct might have to get readjusted if we update cosmrs!!
    pub struct CosmosFee {
        amount: Vec<Coin>,
        gas_limit: u64,
        payer: Option<String>,
        granter: Option<String>,
    }

    // Note: I've got a feeling this one will bite us hard at some point...
    #[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
    #[ts(export_to = "ts-packages/types/src/types/rust/Coin.ts")]
    // this should corresponds to cosmrs::Coin
    // IMPORTANT NOTE: this should work as of cosmrs 0.7.1 due to their `FromStr` implementations
    // on the type. The below struct might have to get readjusted if we update cosmrs!!
    pub struct Coin {
        denom: String,
        // this is not entirely true, but for the purposes
        // of ts_rs, it's sufficient for the time being
        amount: u64,
    }
}
