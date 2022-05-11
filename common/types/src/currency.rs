use crate::error::TypesError;
use cosmrs::Decimal as CosmosDecimal;
use cosmrs::Denom as CosmosDenom;
use cosmwasm_std::Coin as CosmWasmCoin;
use cosmwasm_std::{Decimal, Uint128};
use itertools::Itertools;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::str::FromStr;
use strum::{Display, EnumString, EnumVariantNames};
use validator_client::nymd::{CosmosCoin, GasPrice};

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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MajorCurrencyAmount {
    pub amount: MajorAmountString,
    pub denom: CurrencyDenom,
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

    pub fn from_cosmrs_coin(coin: &CosmosCoin) -> Result<MajorCurrencyAmount, TypesError> {
        MajorCurrencyAmount::from_cosmrs_decimal_and_denom(coin.amount, coin.denom.to_string())
    }

    pub fn from_minor_uint128_and_denom(
        amount_minor: Uint128,
        denom_minor: &str,
    ) -> Result<MajorCurrencyAmount, TypesError> {
        MajorCurrencyAmount::from_minor_decimal_and_denom(
            Decimal::from_atomics(amount_minor, 0)?,
            denom_minor,
        )
    }

    pub fn from_minor_decimal_and_denom(
        amount_minor: Decimal,
        denom_minor: &str,
    ) -> Result<MajorCurrencyAmount, TypesError> {
        if !(denom_minor.starts_with('u') || denom_minor.starts_with('U')) {
            return Err(TypesError::InvalidDenom(denom_minor.to_string()));
        }
        let major = amount_minor / Uint128::from(1_000_000u64);
        if let Ok(denom) = CurrencyDenom::from_str(&denom_minor[1..].to_string()) {
            return Ok(MajorCurrencyAmount {
                amount: MajorAmountString(major.to_string()),
                denom,
            });
        }
        Err(TypesError::InvalidDenom(denom_minor.to_string()))
    }
    pub fn from_decimal_and_denom(
        amount: Decimal,
        denom: String,
    ) -> Result<MajorCurrencyAmount, TypesError> {
        if denom.starts_with('u') || denom.starts_with('U') {
            return MajorCurrencyAmount::from_minor_decimal_and_denom(amount, &denom);
        }
        if let Ok(denom) = CurrencyDenom::from_str(denom.as_str()) {
            return Ok(MajorCurrencyAmount {
                amount: MajorAmountString(amount.to_string()),
                denom,
            });
        }
        Err(TypesError::InvalidDenom(denom))
    }
    pub fn from_cosmrs_decimal_and_denom(
        amount: CosmosDecimal,
        denom: String,
    ) -> Result<MajorCurrencyAmount, TypesError> {
        if denom.starts_with('u') || denom.starts_with('U') {
            return match Decimal::from_str(&amount.to_string()) {
                Ok(amount) => MajorCurrencyAmount::from_minor_decimal_and_denom(amount, &denom),
                Err(_e) => Err(TypesError::InvalidAmount(amount.to_string())),
            };
        }

        if let Ok(denom) = CurrencyDenom::from_str(denom.as_str()) {
            return Ok(MajorCurrencyAmount {
                amount: MajorAmountString(amount.to_string()),
                denom,
            });
        }
        Err(TypesError::InvalidDenom(denom))
    }

    pub fn into_cosmos_coin(self) -> Result<CosmosCoin, TypesError> {
        match CosmosDecimal::from_str(&self.amount.0) {
            Ok(amount) => Ok(CosmosCoin {
                amount,
                denom: CosmosDenom::from_str(&self.denom.to_string().to_lowercase())?,
            }),
            Err(e) => Err(e.into()),
        }
    }

    pub fn to_minor_uint128(&self) -> Result<Uint128, TypesError> {
        if self.amount.0.contains('.') {
            // has a decimal point (Cosmos assumes "." is the decimal separator)
            let parts = self.amount.0.split('.');
            let str = parts.collect_vec();
            if str.is_empty() || str.len() > 2 {
                return Err(TypesError::InvalidAmount("Amount is invalid".to_string()));
            }
            if str.len() == 2 {
                // has a decimal, so check decimal places first
                if str[1].len() > 6 {
                    return Err(TypesError::InvalidDenom(
                        "Amount is invalid, only 6 decimal places of precision are allowed"
                            .to_string(),
                    ));
                }

                // so multiple whole part by 1e6 and add decimal part
                let whole_part = Uint128::from_str(str[0])? * Uint128::from(1_000_000u64);

                // TODO: has Rust got anything that deals with fixed point values, or parsing from format strings? Leading zeroes are causing issues
                return match format!("0.{}", str[1]).parse::<f64>() {
                    Ok(decimal_part_float) => {
                        // this makes an assumption that 6 decimal places of f64 can never lose precision
                        let truncated = (decimal_part_float * 1_000_000.).trunc() as u32;
                        let decimal_part = Uint128::from(truncated);
                        let sum = whole_part + decimal_part;
                        Ok(sum)
                    }
                    Err(_e) => Err(TypesError::InvalidAmount(
                        "Amount decimal part is invalid".to_string(),
                    )),
                };
            }
        }

        let major = Uint128::from_str(&self.amount.0)?;
        let scaled = major * Uint128::new(1_000_000u128);
        Ok(scaled)
    }

    pub fn denom_to_string(&self) -> String {
        self.denom.to_string()
    }

    pub fn into_minor_cosmos_coin(self) -> Result<CosmosCoin, TypesError> {
        let denom = format!("u{}", self.denom_to_string().to_lowercase());
        let minor = self.to_minor_uint128()?;
        let amount = minor.to_string();
        Ok(CosmosCoin {
            amount: CosmosDecimal::from_str(&amount)?,
            denom: CosmosDenom::from_str(&denom)?,
        })
    }

    pub fn into_cosmwasm_coin(self) -> Result<CosmWasmCoin, TypesError> {
        Ok(CosmWasmCoin {
            denom: self.denom.to_string().to_lowercase(),
            amount: Uint128::try_from(self.amount.0.as_str())?,
        })
    }

    pub fn into_minor_cosmwasm_coin(self) -> Result<CosmWasmCoin, TypesError> {
        let denom = format!("u{}", self.denom_to_string().to_lowercase());
        let amount = self.to_minor_uint128()?;
        let amount = Uint128::from_str(&amount.to_string())?;
        Ok(CosmWasmCoin { denom, amount })
    }
}

impl Display for MajorCurrencyAmount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.amount.0, self.denom)
    }
}

impl TryFrom<GasPrice> for MajorCurrencyAmount {
    type Error = TypesError;

    fn try_from(g: GasPrice) -> Result<Self, Self::Error> {
        MajorCurrencyAmount::from_decimal_and_denom(g.amount, g.denom.to_string())
    }
}

impl TryFrom<CosmosCoin> for MajorCurrencyAmount {
    type Error = TypesError;

    fn try_from(c: CosmosCoin) -> Result<Self, Self::Error> {
        MajorCurrencyAmount::from_cosmrs_decimal_and_denom(c.amount, c.denom.to_string())
    }
}

impl TryFrom<CosmWasmCoin> for MajorCurrencyAmount {
    type Error = TypesError;

    fn try_from(c: CosmWasmCoin) -> Result<Self, Self::Error> {
        // note: there are always 0 decimals of precision because:
        // - for major values, only whole values can be represented, e.g. 1 NYM, 1000 NYM
        // - for minor values, again only whole values are represented, e.g. 1 UNYM, 1000 UNYM
        match Decimal::from_atomics(c.amount, 0) {
            Ok(amount) => Ok(MajorCurrencyAmount::from_decimal_and_denom(
                amount, c.denom,
            )?),
            Err(_) => Err(TypesError::InvalidAmount(c.to_string())),
        }
    }
}

impl Add for MajorCurrencyAmount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // TODO: fix up error checking
        let arg1 = Decimal::from_str(&self.amount.0).unwrap_or(Decimal::zero());
        let arg2 = Decimal::from_str(&rhs.amount.0).unwrap_or(Decimal::zero());
        MajorCurrencyAmount::from_decimal_and_denom(arg1 + arg2, self.denom_to_string())
            .unwrap_or_else(|_| MajorCurrencyAmount::zero(&self.denom))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmrs::Coin as CosmosCoin;
    use cosmrs::Decimal;
    use cosmrs::Denom as CosmosDenom;
    use cosmwasm_std::Coin as CosmWasmCoin;
    use cosmwasm_std::Decimal as CosmWasmDecimal;
    use serde_json::json;
    use std::convert::TryFrom;
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
    fn minor_cosmos_coin_to_major_currency() {
        let cosmos_coin = CosmosCoin {
            amount: CosmosDecimal::from(1u64),
            denom: CosmosDenom::from_str("unym").unwrap(),
        };
        let c = MajorCurrencyAmount::from_cosmrs_coin(&cosmos_coin).unwrap();
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
            CosmWasmDecimal::from_atomics(coin.amount.clone(), 6)
                .unwrap()
                .to_string()
        );
        let c: MajorCurrencyAmount = coin.try_into().unwrap();
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
            CosmWasmDecimal::from_atomics(coin.amount.clone(), 6)
                .unwrap()
                .to_string()
        );
        let c: MajorCurrencyAmount = coin.try_into().unwrap();
        assert_eq!(c, MajorCurrencyAmount::new("1", CurrencyDenom::Nym));
    }

    #[test]
    fn major_cosmwasm_coin_to_major_currency() {
        let coin = CosmWasmCoin {
            amount: Uint128::from(1u64),
            denom: "nym".to_string(),
        };
        println!(
            "from_atomics = {:?}",
            CosmWasmDecimal::from_atomics(coin.amount.clone(), 6)
                .unwrap()
                .to_string()
        );
        let c: MajorCurrencyAmount = coin.try_into().unwrap();
        assert_eq!(c, MajorCurrencyAmount::new("1", CurrencyDenom::Nym));
    }

    #[test]
    fn major_currency_to_minor_cosmos_coin() {
        let expected_cosmos_coin = CosmosCoin {
            amount: CosmosDecimal::from(1u64),
            denom: CosmosDenom::from_str("unym").unwrap(),
        };
        let c = MajorCurrencyAmount::new("0.000001", CurrencyDenom::Nym);
        let minor_cosmos_coin = c.into_minor_cosmos_coin().unwrap();
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
        let minor_cosmos_coin = c.into_minor_cosmos_coin().unwrap();
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
        let c = MajorCurrencyAmount::from_cosmrs_coin(&cosmos_coin).unwrap();
        assert_eq!(c.to_string(), "0.000001 NYM");
    }

    #[test]
    fn denom_to_string() {
        let c = MajorCurrencyAmount::new("1", CurrencyDenom::Nym);
        let denom = c.denom_to_string();
        assert_eq!(denom, "NYM".to_string());
    }

    #[test]
    fn to_minor_one_unym() {
        let c = MajorCurrencyAmount::new("1", CurrencyDenom::Nym);
        let minor = c.to_minor_uint128().unwrap();
        assert_eq!("1000000", minor.to_string());
    }

    #[test]
    fn to_minor() {
        let amounts = vec![
            ("1000000", "1000000000000"),
            ("1", "1000000"),
            ("0.000001", "1"),
        ];

        for amount in amounts {
            let c = MajorCurrencyAmount::new(amount.0, CurrencyDenom::Nym);
            let minor = c.to_minor_uint128().unwrap();
            assert_eq!(amount.1, minor.to_string());
        }
    }

    #[test]
    fn to_minor_errors_expected() {
        let bad_amounts = vec![
            "0.0000001", // because there are more than 6 decimals, it gets truncated
            "0.0000009999999999999999999999999999999999", // would overflow
        ];

        for bad_amount in bad_amounts {
            let c = MajorCurrencyAmount::new(bad_amount, CurrencyDenom::Nym);
            assert!(matches!(
                c.to_minor_uint128().unwrap_err(),
                TypesError::InvalidDenom { .. }
            ));
        }
    }

    fn amounts() -> Vec<&'static str> {
        vec![
            "1",
            "10",
            "100",
            "1000",
            "10000",
            "100000",
            "10000000",
            "100000000",
            "1000000000",
            "10000000000",
            "100000000000",
            "1000000000000",
            "10000000000000",
            "100000000000000",
            "1000000000000000",
            "10000000000000000",
            "100000000000000000",
            "1000000000000000000",
        ]
    }

    #[test]
    fn major_currency_amount_into_cosmos_coin() {
        for amount in amounts() {
            let c = MajorCurrencyAmount::new(amount, CurrencyDenom::Nym);
            let coin: CosmosCoin = c.into_cosmos_coin().unwrap();
            assert_eq!(
                coin,
                CosmosCoin {
                    amount: Decimal::from_str(amount).unwrap(),
                    denom: CosmosDenom::from_str("nym").unwrap()
                }
            );
        }
    }

    #[test]
    fn major_currency_amount_into_cosmwasm_coin() {
        for amount in amounts() {
            let c = MajorCurrencyAmount::new(amount, CurrencyDenom::Nym);
            let coin: CosmWasmCoin = c.into_cosmwasm_coin().unwrap();
            assert_eq!(
                coin,
                CosmWasmCoin {
                    amount: Uint128::try_from(amount).unwrap(),
                    denom: "nym".to_string(),
                }
            );
        }
    }

    #[test]
    fn major_currency_amount_from_gas_price() {
        assert_eq!(
            MajorCurrencyAmount::try_from(GasPrice::from_str("42unym").unwrap()).unwrap(),
            MajorCurrencyAmount {
                amount: MajorAmountString("0.000042".to_string()),
                denom: CurrencyDenom::Nym,
            }
        );

        assert_eq!(
            MajorCurrencyAmount::try_from(GasPrice::from_str("42nym").unwrap()).unwrap(),
            MajorCurrencyAmount {
                amount: MajorAmountString("42".to_string()),
                denom: CurrencyDenom::Nym,
            }
        );

        assert_eq!(
            MajorCurrencyAmount::try_from(GasPrice::from_str("42unymt").unwrap()).unwrap(),
            MajorCurrencyAmount {
                amount: MajorAmountString("0.000042".to_string()),
                denom: CurrencyDenom::Nymt,
            }
        );

        assert_eq!(
            MajorCurrencyAmount::try_from(GasPrice::from_str("42nymt").unwrap()).unwrap(),
            MajorCurrencyAmount {
                amount: MajorAmountString("42".to_string()),
                denom: CurrencyDenom::Nymt,
            }
        );
    }
}
