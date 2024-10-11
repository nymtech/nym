use crate::error::TypesError;
use cosmwasm_std::Fraction;
use cosmwasm_std::{Decimal, Uint128};
use nym_config::defaults::{DenomDetails, DenomDetailsOwned, NymNetworkDetails};
use nym_validator_client::nyxd::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use strum::{Display, EnumString, VariantNames};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/CurrencyDenom.ts"
    )
)]
#[cfg_attr(feature = "generate-ts", ts(rename_all = "lowercase"))]
#[derive(
    Display,
    Default,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    EnumString,
    VariantNames,
    PartialEq,
    Eq,
    JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum CurrencyDenom {
    #[strum(ascii_case_insensitive)]
    #[default]
    Unknown,
    #[strum(ascii_case_insensitive)]
    Nym,
    #[strum(ascii_case_insensitive)]
    Nymt,
    #[strum(ascii_case_insensitive)]
    Nyx,
    #[strum(ascii_case_insensitive)]
    Nyxt,
}

pub type Denom = String;

#[derive(Debug, Default)]
pub struct RegisteredCoins(HashMap<Denom, CoinMetadata>);

impl RegisteredCoins {
    pub fn default_denoms(network: &NymNetworkDetails) -> Self {
        let mut network_coins = HashMap::new();
        network_coins.insert(
            network.chain_details.mix_denom.base.clone(),
            network.chain_details.mix_denom.clone().into(),
        );
        network_coins.insert(
            network.chain_details.stake_denom.base.clone(),
            network.chain_details.stake_denom.clone().into(),
        );
        RegisteredCoins(network_coins)
    }

    pub fn insert(&mut self, denom: Denom, metadata: CoinMetadata) -> Option<CoinMetadata> {
        self.0.insert(denom, metadata)
    }

    pub fn remove(&mut self, denom: &Denom) -> Option<CoinMetadata> {
        self.0.remove(denom)
    }

    pub fn attempt_create_display_coin_from_base_dec_amount(
        &self,
        denom: &Denom,
        base_amount: Decimal,
    ) -> Result<DecCoin, TypesError> {
        for registered_coin in self.0.values() {
            if let Some(exponent) = registered_coin.get_exponent(denom) {
                // if this fails it means we haven't registered our display denom which honestly should never be the case
                // unless somebody is rocking their own custom network
                let display_exponent = registered_coin
                    .get_exponent(&registered_coin.display)
                    .ok_or_else(|| TypesError::UnknownCoinDenom(denom.clone()))?;

                return match exponent.cmp(&display_exponent) {
                    Ordering::Greater => {
                        // we need to scale up, unlikely to ever be needed but included for completion sake,
                        // for example if we decided to created knym with exponent 9 and wanted to convert to nym with exponent 6
                        Ok(DecCoin {
                            denom: denom.into(),
                            amount: try_scale_up_decimal(base_amount, exponent - display_exponent)?,
                        })
                    }
                    // we're already in the display denom
                    Ordering::Equal => Ok(DecCoin {
                        denom: denom.into(),
                        amount: base_amount,
                    }),
                    Ordering::Less => {
                        // we need to scale down, the most common case, for example we're in base unym with exponent 0 and want to convert to nym with exponent 6
                        Ok(DecCoin {
                            denom: denom.into(),
                            amount: try_scale_down_decimal(
                                base_amount,
                                display_exponent - exponent,
                            )?,
                        })
                    }
                };
            }
        }

        Err(TypesError::UnknownCoinDenom(denom.clone()))
    }

    pub fn attempt_convert_to_base_coin(&self, coin: DecCoin) -> Result<Coin, TypesError> {
        // check if this is already in the base denom
        if self.0.contains_key(&coin.denom) {
            // if we're converting a base DecCoin it CANNOT fail, unless somebody is providing
            // bullshit data on purpose : )
            return coin.try_into();
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
                        // we need to scale up, unlikely to ever be needed but included for completion sake,
                        // for example if we decided to created knym with exponent 9 and wanted to convert to nym with exponent 6
                        Ok(DecCoin::new_scaled_up(
                            coin.amount,
                            &registered_coin.display,
                            exponent - display_exponent,
                        )?)
                    }
                    // we're already in the display denom
                    Ordering::Equal => Ok(coin.into()),
                    Ordering::Less => {
                        // we need to scale down, the most common case, for example we're in base unym with exponent 0 and want to convert to nym with exponent 6
                        Ok(DecCoin::new_scaled_down(
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

impl From<DenomDetailsOwned> for CoinMetadata {
    fn from(denom_details: DenomDetailsOwned) -> Self {
        CoinMetadata::new(
            vec![
                DenomUnit::new(denom_details.base.clone(), 0),
                DenomUnit::new(
                    denom_details.display.clone(),
                    denom_details.display_exponent,
                ),
            ],
            denom_details.base,
            denom_details.display,
        )
    }
}

// tries to semi-replicate cosmos-sdk's DecCoin for being able to handle tokens with decimal amounts
// https://github.com/cosmos/cosmos-sdk/blob/v0.45.4/types/dec_coin.go
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "ts-packages/types/src/types/rust/DecCoin.ts")
)]
pub struct DecCoin {
    #[cfg_attr(feature = "generate-ts", ts(as = "CurrencyDenom"))]
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

impl Display for DecCoin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.amount, self.denom)
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

    #[test]
    fn converting_to_display() {
        let reg = RegisteredCoins::default_denoms(&NymNetworkDetails::new_mainnet());
        let values = vec![
            (1u128, "0.000001"),
            (10u128, "0.00001"),
            (100u128, "0.0001"),
            (1000u128, "0.001"),
            (10000u128, "0.01"),
            (100000u128, "0.1"),
            (1000000u128, "1"),
            (1234567u128, "1.234567"),
            (123456700u128, "123.4567"),
        ];

        for (raw, expected) in values {
            let coin = Coin::new(
                raw,
                NymNetworkDetails::new_mainnet()
                    .chain_details
                    .mix_denom
                    .base
                    .clone(),
            );
            let display = reg.attempt_convert_to_display_dec_coin(coin).unwrap();
            assert_eq!(
                NymNetworkDetails::new_mainnet()
                    .chain_details
                    .mix_denom
                    .display,
                display.denom
            );
            assert_eq!(expected, display.amount.to_string());
        }
    }

    #[test]
    fn converting_to_base() {
        let reg = RegisteredCoins::default_denoms(&NymNetworkDetails::new_mainnet());
        let values = vec![
            (1u128, "0.000001"),
            (10u128, "0.00001"),
            (100u128, "0.0001"),
            (1000u128, "0.001"),
            (10000u128, "0.01"),
            (100000u128, "0.1"),
            (1000000u128, "1"),
            (1234567u128, "1.234567"),
            (123456700u128, "123.4567"),
        ];

        for (expected, raw_display) in values {
            let coin = DecCoin {
                denom: NymNetworkDetails::new_mainnet()
                    .chain_details
                    .mix_denom
                    .display
                    .clone(),
                amount: raw_display.parse().unwrap(),
            };
            let base = reg.attempt_convert_to_base_coin(coin).unwrap();
            assert_eq!(
                NymNetworkDetails::new_mainnet()
                    .chain_details
                    .mix_denom
                    .base,
                base.denom
            );
            assert_eq!(expected, base.amount);
        }
    }
}
