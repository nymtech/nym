use crate::error::TypesError;
use config::defaults::all::Network;
use config::defaults::DenomDetails;
use cosmwasm_std::Fraction;
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use validator_client::nymd::Coin;

pub type Denom = String;

#[derive(Debug, Default)]
pub struct RegisteredCoins(HashMap<Denom, CoinMetadata>);

impl RegisteredCoins {
    pub fn default_denoms(network: Network) -> Self {
        let mut network_coins = HashMap::new();
        network_coins.insert(
            network.mix_denom().base.into(),
            (*network.mix_denom()).into(),
        );
        network_coins.insert(
            network.stake_denom().base.into(),
            (*network.stake_denom()).into(),
        );
        RegisteredCoins(network_coins)
    }

    pub fn attempt_convert_to_base_coin(&self, coin: DecCoin) -> Result<Coin, TypesError> {
        // check if this is already in the base denom
        if self.0.contains_key(&coin.denom) {
            // if we're converting a base DecCoin it CANNOT fail, unless somebody is providing
            // bullshit data on purpose : )
            return Ok(coin.try_into()?);
        } else {
            // TODO: this kinda suggests we may need a better data structure
            for registered_coin in self.0.values() {
                if let Some(exponent) = registered_coin.get_exponent(&coin.denom) {
                    let amount = try_convert_decimal_to_u128(coin.try_scale_up_value(exponent)?)?;
                    return Ok(Coin::new(amount, &registered_coin.base));
                }
            }
        }
        Err(TypesError::UnknownCoinDenom(coin.denom))
    }

    pub fn attempt_convert_to_display_dec_coin(&self, coin: Coin) -> Result<DecCoin, TypesError> {
        for registered_coin in self.0.values() {
            if let Some(exponent) = registered_coin.get_exponent(&coin.denom) {
                // if this fails it means we haven't registered our display denom which honestly should never be the case
                // unless somebody is rocking their own custom network
                let display_exponent = registered_coin
                    .get_exponent(&registered_coin.display)
                    .ok_or_else(|| TypesError::UnknownCoinDenom(coin.denom.clone()))?;

                return match exponent.cmp(&display_exponent) {
                    Ordering::Greater => {
                        // we need to scale down, unlikely to ever be needed but included for completion sake,
                        // for example if we decided to created knym with exponent 9 and wanted to convert to nym with exponent 6
                        Ok(DecCoin::new_scaled_down(
                            coin.amount,
                            &registered_coin.display,
                            exponent - display_exponent,
                        )?)
                    }
                    // we're already in the display denom
                    Ordering::Equal => Ok(coin.into()),
                    Ordering::Less => {
                        // we need to scale up, the most common case, for example we're in base unym with exponent 0 and want to convert to nym with exponent 6
                        Ok(DecCoin::new_scaled_up(
                            coin.amount,
                            &registered_coin.display,
                            display_exponent - exponent,
                        )?)
                    }
                };
            }
        }

        Err(TypesError::UnknownCoinDenom(coin.denom))
    }
}

// TODO: should this live here?
// attempts to replicate cosmos-sdk's coin metadata
// https://docs.cosmos.network/master/architecture/adr-024-coin-metadata.html
// this way we could more easily handle multiple coin types simultaneously (like nym/nyx/nymt/nyx + local currencies)
#[derive(Debug)]
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

#[derive(Debug)]
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

    pub fn zero<S: Into<String>>(denom: S) -> Self {
        DecCoin {
            denom: denom.into(),
            amount: Decimal::zero(),
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

// TODO: adjust the implementation to as required by @MS or @FT
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
