use crate::error::TypesError;
use config::defaults::DenomDetails;
use cosmrs::Denom as CosmosDenom;
use cosmwasm_std::{Coin as CosmWasmCoin, Fraction};
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use std::ops::{Add, Mul};
use std::str::FromStr;
use strum::{Display, EnumString, EnumVariantNames};
use validator_client::nymd::{Coin, CosmosCoin};

pub type Denom = String;

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
#[deprecated]
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
#[deprecated]
pub struct MajorAmountString(String); // see https://github.com/Aleph-Alpha/ts-rs/issues/51 for exporting type aliases

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/Currency.ts")
)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[deprecated]
pub struct MajorCurrencyAmount {
    pub amount: MajorAmountString,
    pub denom: CurrencyDenom,
}

// TODO: should this live here?
// attempts to replicate cosmos-sdk's coin metadata
// https://docs.cosmos.network/master/architecture/adr-024-coin-metadata.html
// this way we could more easily handle multiple coin types simultaneously (like nym/nyx/nymt/nyx + local currencies)
pub struct DenomUnit {
    pub denom: Denom,
    pub exponent: u32,
    // pub aliases: Vec<String>,
}

impl DenomUnit {
    pub fn new(denom: Denom, exponent: u32) -> Self {
        DenomUnit { denom, exponent }
    }
}

pub struct CoinMetadata {
    pub denom_units: Vec<DenomUnit>,
    pub base: Denom,
    pub display: Denom,
}

impl CoinMetadata {
    pub fn new(denom_units: Vec<DenomUnit>, base: Denom, display: Denom) -> Self {
        CoinMetadata {
            denom_units,
            base,
            display,
        }
    }

    pub fn get_exponent(&self, denom: &str) -> Option<u32> {
        self.denom_units
            .iter()
            .find(|denom_unit| denom_unit.denom == denom)
            .map(|denom_unit| denom_unit.exponent)
    }
}

impl From<DenomDetails> for CoinMetadata {
    fn from(denom_details: DenomDetails) -> Self {
        CoinMetadata::new(
            vec![
                DenomUnit::new(denom_details.base.into(), 0),
                DenomUnit::new(denom_details.display.into(), denom_details.display_exponent),
            ],
            denom_details.base.into(),
            denom_details.display.into(),
        )
    }
}

// tries to semi-replicate cosmos-sdk's DecCoin for being able to handle tokens with decimal amounts
// https://github.com/cosmos/cosmos-sdk/blob/v0.45.4/types/dec_coin.go
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/DecCoin.ts")
)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DecCoin {
    pub denom: Denom,
    // Decimal is already serialized to string and using string in its schema, so lets also go straight to string for ts_rs
    // todo: is `Decimal` the correct type to use? Do we want to depend on cosmwasm_std here?
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub amount: Decimal,
}

impl DecCoin {
    pub fn new_base<S: Into<String>>(amount: impl Into<Uint128>, denom: S) -> Self {
        DecCoin {
            denom: denom.into(),
            amount: Decimal::from_atomics(amount, 0).unwrap(),
        }
    }

    pub fn new_scaled_up<S: Into<String>>(
        base_amount: impl Into<Uint128>,
        denom: S,
        exponent: u32,
    ) -> Result<Self, TypesError> {
        let base_amount = Decimal::from_atomics(base_amount, 0).unwrap();
        Ok(DecCoin {
            denom: denom.into(),
            amount: try_scale_up_decimal(base_amount, exponent)?,
        })
    }

    pub fn new_scaled_down<S: Into<String>>(
        base_amount: impl Into<Uint128>,
        denom: S,
        exponent: u32,
    ) -> Result<Self, TypesError> {
        let base_amount = Decimal::from_atomics(base_amount, 0).unwrap();
        Ok(DecCoin {
            denom: denom.into(),
            amount: try_scale_down_decimal(base_amount, exponent)?,
        })
    }

    pub fn try_scale_down_value(&self, exponent: u32) -> Result<Decimal, TypesError> {
        try_scale_down_decimal(self.amount, exponent)
    }

    pub fn try_scale_up_value(&self, exponent: u32) -> Result<Decimal, TypesError> {
        try_scale_up_decimal(self.amount, exponent)
    }
}

// TODO: should thoese live here?
pub fn try_scale_down_decimal(dec: Decimal, exponent: u32) -> Result<Decimal, TypesError> {
    let rhs = 10u128
        .checked_pow(exponent)
        .ok_or(TypesError::UnsupportedExponent(exponent))?;
    let denominator = dec
        .denominator()
        .checked_mul(rhs.into())
        .map_err(|_| TypesError::UnsupportedExponent(exponent))?;

    Ok(Decimal::from_ratio(dec.numerator(), denominator))
}

pub fn try_scale_up_decimal(dec: Decimal, exponent: u32) -> Result<Decimal, TypesError> {
    let rhs = 10u128
        .checked_pow(exponent)
        .ok_or(TypesError::UnsupportedExponent(exponent))?;
    let denominator = dec
        .denominator()
        .checked_div(rhs.into())
        .map_err(|_| TypesError::UnsupportedExponent(exponent))?;

    Ok(Decimal::from_ratio(dec.numerator(), denominator))
}

pub fn try_convert_decimal_to_u128(dec: Decimal) -> Result<u128, TypesError> {
    let whole = dec.numerator() / dec.denominator();

    // unwrap is fine as we're not dividing by zero here
    let fractional = (dec.numerator()).checked_rem(dec.denominator()).unwrap();

    // we cannot convert as we'd lose our decimal places
    // (for example if somebody attempted to represent our gas price (WHICH YOU SHOULDN'T DO) as DecCoin)
    if fractional != Uint128::zero() {
        return Err(TypesError::LossyCoinConversion);
    }
    Ok(whole.u128())
}

impl Display for DecCoin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.amount, self.denom)
    }
}

impl From<Coin> for DecCoin {
    fn from(coin: Coin) -> Self {
        DecCoin::new_base(coin.amount, coin.denom)
    }
}

// this conversion assumes same denomination
impl TryFrom<DecCoin> for Coin {
    type Error = TypesError;

    fn try_from(value: DecCoin) -> Result<Self, Self::Error> {
        Ok(Coin {
            amount: try_convert_decimal_to_u128(value.try_scale_down_value(0)?)?,
            denom: value.denom,
        })
    }
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
            CosmWasmDecimal::from_atomics(coin.amount.clone(), 6)
                .unwrap()
                .to_string()
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
            CosmWasmDecimal::from_atomics(coin.amount.clone(), 6)
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
    fn dec_value_scale_down() {
        let dec = DecCoin {
            denom: "foo".to_string(),
            amount: "1234007000".parse().unwrap(),
        };

        assert_eq!(
            "1234007000".parse::<Decimal>().unwrap(),
            dec.try_scale_down_value(0).unwrap()
        );
        assert_eq!(
            "123400700".parse::<Decimal>().unwrap(),
            dec.try_scale_down_value(1).unwrap()
        );
        assert_eq!(
            "12340070".parse::<Decimal>().unwrap(),
            dec.try_scale_down_value(2).unwrap()
        );
        assert_eq!(
            "123400.7".parse::<Decimal>().unwrap(),
            dec.try_scale_down_value(4).unwrap()
        );

        let dec = DecCoin {
            denom: "foo".to_string(),
            amount: "10000000000".parse().unwrap(),
        };

        assert_eq!(
            "100".parse::<Decimal>().unwrap(),
            dec.try_scale_down_value(8).unwrap()
        );
        assert_eq!(
            "1".parse::<Decimal>().unwrap(),
            dec.try_scale_down_value(10).unwrap()
        );
        assert_eq!(
            "0.01".parse::<Decimal>().unwrap(),
            dec.try_scale_down_value(12).unwrap()
        );
    }

    #[test]
    fn dec_value_scale_up() {
        let dec = DecCoin {
            denom: "foo".to_string(),
            amount: "1234.56".parse().unwrap(),
        };

        assert_eq!(
            "1234.56".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(0).unwrap()
        );
        assert_eq!(
            "12345.6".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(1).unwrap()
        );
        assert_eq!(
            "123456".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(2).unwrap()
        );
        assert_eq!(
            "1234560".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(3).unwrap()
        );
        assert_eq!(
            "12345600".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(4).unwrap()
        );

        let dec = DecCoin {
            denom: "foo".to_string(),
            amount: "0.00000123".parse().unwrap(),
        };

        assert_eq!(
            "0.0000123".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(1).unwrap()
        );
        assert_eq!(
            "0.000123".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(2).unwrap()
        );
        assert_eq!(
            "123".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(8).unwrap()
        );
        assert_eq!(
            "1230".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(9).unwrap()
        );
        assert_eq!(
            "12300".parse::<Decimal>().unwrap(),
            dec.try_scale_up_value(10).unwrap()
        );
    }

    #[test]
    fn dec_to_u128() {
        todo!()
    }

    #[test]
    fn coin_to_dec_coin() {
        let coin = Coin::new(123, "foo");
        let dec = DecCoin::from(coin.clone());
        assert_eq!(coin.denom, dec.denom);
        assert_eq!(dec.amount, Decimal::from_atomics(coin.amount, 0).unwrap());
    }

    #[test]
    fn dec_coin_to_coin() {
        let dec = DecCoin {
            denom: "foo".to_string(),
            amount: "123".parse().unwrap(),
        };
        let coin = Coin::try_from(dec.clone()).unwrap();
        assert_eq!(dec.denom, coin.denom);
        assert_eq!(coin.amount, 123u128);
    }
}
