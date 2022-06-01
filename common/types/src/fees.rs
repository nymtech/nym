use crate::currency::DecCoin;
use serde::{Deserialize, Serialize};
use validator_client::nymd::Fee;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/FeeDetails.ts")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDetails {
    // expected to be used by the wallet in order to display detailed fee information to the user
    pub amount: Option<DecCoin>,
    #[cfg_attr(feature = "generate-ts", ts(skip))]
    pub fee: Fee,
}

impl FeeDetails {
    pub fn new(amount: Option<DecCoin>, fee: Fee) -> Self {
        FeeDetails { amount, fee }
    }
}
