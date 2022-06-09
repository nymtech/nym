use serde::{Deserialize, Serialize};

use validator_client::nymd::Fee;

use crate::currency::MajorCurrencyAmount;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/FeeDetails.ts")
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDetails {
    // expected to be used by the wallet in order to display detailed fee information to the user
    pub amount: Option<MajorCurrencyAmount>,
    #[cfg_attr(feature = "generate-ts", ts(skip))]
    pub fee: Fee,
}
