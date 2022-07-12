use crate::currency::DecCoin;
use serde::{Deserialize, Serialize};
use validator_client::nymd::Fee;

#[cfg(feature = "generate-ts")]
use ts_rs::{Dependency, TS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDetails {
    // expected to be used by the wallet in order to display detailed fee information to the user
    pub amount: Option<DecCoin>,
    pub fee: Fee,
}

impl FeeDetails {
    pub fn new(amount: Option<DecCoin>, fee: Fee) -> Self {
        FeeDetails { amount, fee }
    }
}

#[cfg(feature = "generate-ts")]
impl TS for FeeDetails {
    const EXPORT_TO: Option<&'static str> = Some("ts-packages/types/src/types/rust/FeeDetails.ts");

    fn decl() -> String {
        format!("type {} = {};", Self::name(), Self::inline())
    }

    fn name() -> String {
        "FeeDetails".into()
    }

    fn inline() -> String {
        "{ amount: DecCoin | null, fee: Fee }".into()
    }

    fn dependencies() -> Vec<Dependency> {
        vec![
            Dependency::from_ty::<DecCoin>().expect("TS was incorrectly defined on `DecCoin`"),
            Dependency::from_ty::<ts_type_helpers::Fee>()
                .expect("TS was incorrectly defined on `ts_type_helpers::Fee`"),
        ]
    }

    fn transparent() -> bool {
        false
    }
}

// this should really be sealed and NEVER EVER used as "normal" types,
// but due to our typescript requirements, we have to expose it to generate
// the types...
#[cfg(feature = "generate-ts")]
pub mod ts_type_helpers {
    use serde::{Deserialize, Serialize};
    use validator_client::nymd::GasAdjustment;

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
