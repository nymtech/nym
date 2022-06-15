use crate::error::TypesError;
use cosmrs::Denom as CosmosDenom;
use cosmwasm_std::Coin as CosmWasmCoin;
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::ops::{Add, Mul};
use std::str::FromStr;
use strum::{Display, EnumString, EnumVariantNames};
use validator_client::nymd::{Coin, CosmosCoin};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/CurrencyDenom.ts")
)]
#[cfg_attr(feature = "generate-ts", ts(rename_all = "UPPERCASE"))]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    EnumString,
    EnumVariantNames,
    PartialEq,
    JsonSchema,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
// TODO: this shouldn't be an enum...
pub enum CurrencyDenom {
    #[strum(ascii_case_insensitive)]
    Nym,
    #[strum(ascii_case_insensitive)]
    Nymt,
    #[strum(ascii_case_insensitive)]
    Nyx,
    #[strum(ascii_case_insensitive)]
    Nyxt,
}

impl CurrencyDenom {
    pub fn parse(value: &str) -> Result<CurrencyDenom, TypesError> {
        let mut denom = value.to_string();
        if denom.starts_with('u') {
            denom = denom[1..].to_string();
        }
        match CurrencyDenom::from_str(&denom) {
            Ok(res) => Ok(res),
            Err(_e) => Err(TypesError::InvalidDenom(value.to_string())),
        }
    }
}

impl TryFrom<CosmosDenom> for CurrencyDenom {
    type Error = TypesError;

    fn try_from(value: CosmosDenom) -> Result<Self, Self::Error> {
        CurrencyDenom::parse(&value.to_string())
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/CurrencyStringMajorAmount.ts")
)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MajorAmountString(String); // see https://github.com/Aleph-Alpha/ts-rs/issues/51 for exporting type aliases

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Currency.ts")
)]
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MajorCurrencyAmount {
    // temporarly going back to original impl to speed up merge
    pub amount: MajorAmountString,
    pub denom: CurrencyDenom,
    // // temporary...
    // #[cfg_attr(feature = "generate-ts", ts(skip))]
    // pub coin: Coin,
}

// impl JsonSchema for MajorCurrencyAmount {
//     fn schema_name() -> String {
//         todo!()
//     }
//
//     fn json_schema(gen: &mut SchemaGenerator) -> Schema {
//         todo!()
//     }
// }

// tries to semi-replicate cosmos-sdk's DecCoin for being able to handle tokens with decimal amounts
// https://github.com/cosmos/cosmos-sdk/blob/v0.45.4/types/dec_coin.go
pub struct DecCoin {
    //
}

impl MajorCurrencyAmount {
    pub fn new(amount: &str, denom: CurrencyDenom) -> MajorCurrencyAmount {
        MajorCurrencyAmount {
            amount: MajorAmountString(amount.to_string()),
            denom,
        }
    }

    pub fn zero(denom: &CurrencyDenom) -> MajorCurrencyAmount {
        MajorCurrencyAmount::new("0", denom.clone())
    }
    //
    // pub fn from_cosmrs_coin(coin: &CosmosCoin) -> Result<MajorCurrencyAmount, TypesError> {
    //     MajorCurrencyAmount::from_cosmrs_decimal_and_denom(coin.amount, coin.denom.to_string())
    // }
    //
    // pub fn from_minor_uint128_and_denom(
    //     amount_minor: Uint128,
    //     denom_minor: &str,
    // ) -> Result<MajorCurrencyAmount, TypesError> {
    //     MajorCurrencyAmount::from_minor_decimal_and_denom(
    //         Decimal::from_atomics(amount_minor, 0)?,
    //         denom_minor,
    //     )
    // }
    //
    // pub fn from_minor_decimal_and_denom(
    //     amount_minor: Decimal,
    //     denom_minor: &str,
    // ) -> Result<MajorCurrencyAmount, TypesError> {
    //     if !(denom_minor.starts_with('u') || denom_minor.starts_with('U')) {
    //         return Err(TypesError::InvalidDenom(denom_minor.to_string()));
    //     }
    //     let major = amount_minor / Uint128::from(1_000_000u64);
    //     if let Ok(denom) = CurrencyDenom::from_str(&denom_minor[1..].to_string()) {
    //         return Ok(MajorCurrencyAmount {
    //             amount: MajorAmountString(major.to_string()),
    //             denom,
    //         });
    //     }
    //     Err(TypesError::InvalidDenom(denom_minor.to_string()))
    // }
    // pub fn from_decimal_and_denom(
    //     amount: Decimal,
    //     denom: String,
    // ) -> Result<MajorCurrencyAmount, TypesError> {
    //     if denom.starts_with('u') || denom.starts_with('U') {
    //         return MajorCurrencyAmount::from_minor_decimal_and_denom(amount, &denom);
    //     }
    //     if let Ok(denom) = CurrencyDenom::from_str(denom.as_str()) {
    //         return Ok(MajorCurrencyAmount {
    //             amount: MajorAmountString(amount.to_string()),
    //             denom,
    //         });
    //     }
    //     Err(TypesError::InvalidDenom(denom))
    // }
    // pub fn from_cosmrs_decimal_and_denom(
    //     amount: CosmosDecimal,
    //     denom: String,
    // ) -> Result<MajorCurrencyAmount, TypesError> {
    //     if denom.starts_with('u') || denom.starts_with('U') {
    //         return match Decimal::from_str(&amount.to_string()) {
    //             Ok(amount) => MajorCurrencyAmount::from_minor_decimal_and_denom(amount, &denom),
    //             Err(_e) => Err(TypesError::InvalidAmount(amount.to_string())),
    //         };
    //     }
    //
    //     if let Ok(denom) = CurrencyDenom::from_str(denom.as_str()) {
    //         return Ok(MajorCurrencyAmount {
    //             amount: MajorAmountString(amount.to_string()),
    //             denom,
    //         });
    //     }
    //     Err(TypesError::InvalidDenom(denom))
    // }
    //
    // pub fn into_cosmos_coin(self) -> CosmosCoin {
    //     self.coin.into()
    // }
    //
    // pub fn to_minor_uint128(&self) -> Result<Uint128, TypesError> {
    //     if self.amount.0.contains('.') {
    //         // has a decimal point (Cosmos assumes "." is the decimal separator)
    //         let parts = self.amount.0.split('.');
    //         let str = parts.collect_vec();
    //         if str.is_empty() || str.len() > 2 {
    //             return Err(TypesError::InvalidAmount("Amount is invalid".to_string()));
    //         }
    //         if str.len() == 2 {
    //             // has a decimal, so check decimal places first
    //             if str[1].len() > 6 {
    //                 return Err(TypesError::InvalidDenom(
    //                     "Amount is invalid, only 6 decimal places of precision are allowed"
    //                         .to_string(),
    //                 ));
    //             }
    //
    //             // so multiple whole part by 1e6 and add decimal part
    //             let whole_part = Uint128::from_str(str[0])? * Uint128::from(1_000_000u64);
    //
    //             // TODO: has Rust got anything that deals with fixed point values, or parsing from format strings? Leading zeroes are causing issues
    //             return match format!("0.{}", str[1]).parse::<f64>() {
    //                 Ok(decimal_part_float) => {
    //                     // this makes an assumption that 6 decimal places of f64 can never lose precision
    //                     let truncated = (decimal_part_float * 1_000_000.).trunc() as u32;
    //                     let decimal_part = Uint128::from(truncated);
    //                     let sum = whole_part + decimal_part;
    //                     Ok(sum)
    //                 }
    //                 Err(_e) => Err(TypesError::InvalidAmount(
    //                     "Amount decimal part is invalid".to_string(),
    //                 )),
    //             };
    //         }
    //     }
    //
    //     let major = Uint128::from_str(&self.amount.0)?;
    //     let scaled = major * Uint128::new(1_000_000u128);
    //     Ok(scaled)
    // }

    // pub fn denom_to_string(&self) -> String {
    //     self.denom.to_string()
    // }
}

impl Display for MajorCurrencyAmount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.amount.0, self.denom)
    }
}

// TODO: cleanup after merge
impl From<CosmosCoin> for MajorCurrencyAmount {
    fn from(c: CosmosCoin) -> Self {
        MajorCurrencyAmount::from(Coin::from(c))
    }
}

impl From<CosmWasmCoin> for MajorCurrencyAmount {
    fn from(c: CosmWasmCoin) -> Self {
        MajorCurrencyAmount::from(Coin::from(c))
    }
}

impl From<Coin> for MajorCurrencyAmount {
    fn from(coin: Coin) -> Self {
        // current assumption: MajorCurrencyAmount is represented as decimal with 6 decimal points
        // unwrap is fine as we haven't exceeded decimal range since our coins are at max 1B in value
        // (this is a weak assumption, but for solving this merge conflict it's good enough temporary workaround)
        let amount = Decimal::from_atomics(coin.amount, 6).unwrap();
        MajorCurrencyAmount {
            amount: MajorAmountString(amount.to_string()),
            denom: CurrencyDenom::parse(&coin.denom).expect("this will go away after the merge..."),
        }
    }
}

// temporary...
impl From<MajorCurrencyAmount> for CosmosCoin {
    fn from(c: MajorCurrencyAmount) -> CosmosCoin {
        let c: Coin = c.into();
        c.into()
    }
}

impl From<MajorCurrencyAmount> for CosmWasmCoin {
    fn from(c: MajorCurrencyAmount) -> CosmWasmCoin {
        let c: Coin = c.into();
        c.into()
    }
}

impl From<MajorCurrencyAmount> for Coin {
    fn from(c: MajorCurrencyAmount) -> Coin {
        let decimal: Decimal = c
            .amount
            .0
            .parse()
            .expect("stringified amount should have been a valid decimal");

        // again, temporary
        let exp = Uint128::new(1000000);
        let val = decimal.mul(exp);

        // again, terrible assumption for denom, but it works temporarily...
        Coin {
            amount: val.u128(),
            denom: format!("u{}", c.denom).to_lowercase(),
        }
    }
}

impl Add for MajorCurrencyAmount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // again, temporary workaround to help with merge
        (Coin::from(self).try_add(&Coin::from(rhs)))
            .expect("provided coins had different denoms")
            .into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmrs::Coin as CosmosCoin;
    use cosmrs::Decimal as CosmosDecimal;
    use cosmrs::Denom as CosmosDenom;
    use cosmwasm_std::Coin as CosmWasmCoin;
    use cosmwasm_std::Decimal as CosmWasmDecimal;
    use serde_json::json;
    use std::str::FromStr;
    use std::string::ToString;

    #[test]
    fn json_to_major_currency_amount() {
        let nym = json!({
            "amount": "1",
            "denom": "NYM"
        });
        let nymt = json!({
            "amount": "1",
            "denom": "NYMT"
        });

        let test_nym_amount = MajorCurrencyAmount::new("1", CurrencyDenom::Nym);
        let test_nymt_amount = MajorCurrencyAmount::new("1", CurrencyDenom::Nymt);

        let nym_amount = serde_json::from_value::<MajorCurrencyAmount>(nym).unwrap();
        let nymt_amount = serde_json::from_value::<MajorCurrencyAmount>(nymt).unwrap();

        assert_eq!(nym_amount, test_nym_amount);
        assert_eq!(nymt_amount, test_nymt_amount);
    }

    #[test]
    fn minor_amount_json_to_major_currency_amount() {
        let one_micro_nym = json!({
            "amount": "0.000001",
            "denom": "NYM"
        });

        let expected_nym_amount = MajorCurrencyAmount::new("0.000001", CurrencyDenom::Nym);
        let actual_nym_amount =
            serde_json::from_value::<MajorCurrencyAmount>(one_micro_nym).unwrap();

        assert_eq!(expected_nym_amount, actual_nym_amount);
    }

    #[test]
    fn denom_from_str() {
        assert_eq!(CurrencyDenom::from_str("nym").unwrap(), CurrencyDenom::Nym);
        assert_eq!(
            CurrencyDenom::from_str("nymt").unwrap(),
            CurrencyDenom::Nymt
        );
        assert_eq!(CurrencyDenom::from_str("NYM").unwrap(), CurrencyDenom::Nym);
        assert_eq!(
            CurrencyDenom::from_str("NYMT").unwrap(),
            CurrencyDenom::Nymt
        );
        assert_eq!(CurrencyDenom::from_str("NyM").unwrap(), CurrencyDenom::Nym);
        assert_eq!(
            CurrencyDenom::from_str("NYmt").unwrap(),
            CurrencyDenom::Nymt
        );

        assert!(matches!(
            CurrencyDenom::from_str("foo").unwrap_err(),
            strum::ParseError::VariantNotFound,
        ));

        // denominations must all be major
        assert!(matches!(
            CurrencyDenom::from_str("unym").unwrap_err(),
            strum::ParseError::VariantNotFound,
        ));
        assert!(matches!(
            CurrencyDenom::from_str("unymt").unwrap_err(),
            strum::ParseError::VariantNotFound,
        ));
    }

    #[test]
    fn to_string() {
        assert_eq!(
            MajorCurrencyAmount::new("1", CurrencyDenom::Nym).to_string(),
            "1 NYM"
        );
        assert_eq!(
            MajorCurrencyAmount::new("1", CurrencyDenom::Nymt).to_string(),
            "1 NYMT"
        );
        assert_eq!(
            MajorCurrencyAmount::new("1000000000000", CurrencyDenom::Nym).to_string(),
            "1000000000000 NYM"
        );
    }

    #[test]
    fn minor_coin_to_major_currency() {
        let cosmos_coin = CosmosCoin {
            amount: CosmosDecimal::from(1u64),
            denom: CosmosDenom::from_str("unym").unwrap(),
        };
        let c = MajorCurrencyAmount::from(cosmos_coin);
        assert_eq!(c, MajorCurrencyAmount::new("0.000001", CurrencyDenom::Nym));
    }

    #[test]
    fn minor_cosmwasm_coin_to_major_currency() {
        let coin = CosmWasmCoin {
            amount: Uint128::from(1u64),
            denom: "unym".to_string(),
        };
        println!(
            "from_atomics = {}",
            CosmWasmDecimal::from_atomics(coin.amount, 6).unwrap()
        );
        let c: MajorCurrencyAmount = coin.into();
        assert_eq!(c, MajorCurrencyAmount::new("0.000001", CurrencyDenom::Nym));
    }

    #[test]
    fn minor_cosmwasm_coin_to_major_currency_2() {
        let coin = CosmWasmCoin {
            amount: Uint128::from(1_000_000u64),
            denom: "unym".to_string(),
        };
        println!(
            "from_atomics = {:?}",
            CosmWasmDecimal::from_atomics(coin.amount, 6)
                .unwrap()
                .to_string()
        );
        let c: MajorCurrencyAmount = coin.into();
        assert_eq!(c, MajorCurrencyAmount::new("1", CurrencyDenom::Nym));
    }

    #[test]
    fn major_currency_to_minor_cosmos_coin() {
        let expected_cosmos_coin = CosmosCoin {
            amount: CosmosDecimal::from(1u64),
            denom: CosmosDenom::from_str("unym").unwrap(),
        };
        let c = MajorCurrencyAmount::new("0.000001", CurrencyDenom::Nym);
        let minor_cosmos_coin = c.into();
        assert_eq!(expected_cosmos_coin, minor_cosmos_coin);
        assert_eq!("unym", minor_cosmos_coin.denom.to_string());
    }

    #[test]
    fn major_currency_to_minor_cosmos_coin_2() {
        let expected_cosmos_coin = CosmosCoin {
            amount: CosmosDecimal::from(1000000u64),
            denom: CosmosDenom::from_str("unym").unwrap(),
        };
        let c = MajorCurrencyAmount::new("1", CurrencyDenom::Nym);
        let minor_cosmos_coin = c.into();
        assert_eq!(expected_cosmos_coin, minor_cosmos_coin);
        assert_eq!("unym", minor_cosmos_coin.denom.to_string());
    }

    #[test]
    fn minor_cosmos_coin_to_major_currency_string() {
        // check minor cosmos coin is converted to major value
        let cosmos_coin = CosmosCoin {
            amount: CosmosDecimal::from(1u64),
            denom: CosmosDenom::from_str("unym").unwrap(),
        };
        let c = MajorCurrencyAmount::from(cosmos_coin);
        assert_eq!(c.to_string(), "0.000001 NYM");
    }

    #[test]
    fn denom_to_string() {
        let c = MajorCurrencyAmount::new("1", CurrencyDenom::Nym);
        let denom = c.denom.to_string();
        assert_eq!(denom, "NYM".to_string());
    }
}
