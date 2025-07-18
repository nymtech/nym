// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::OverflowError;
use cosmwasm_std::Uint128;
use cosmwasm_std::{Decimal, Fraction};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::fmt::{self, Display, Formatter};
use std::ops::{Deref, Mul};
use std::str::FromStr;
use thiserror::Error;

/// Ed25519 public key strinfified into base58.
pub type IdentityKey = String;
pub type IdentityKeyRef<'a> = &'a str;

pub fn truncate_decimal(amount: Decimal) -> Uint128 {
    Uint128::new(1).mul_floor(amount)
}

#[derive(Error, Debug)]
pub enum ContractsCommonError {
    #[error("Provided percent value ({0}) is greater than 100%")]
    InvalidPercent(String),

    #[error("{source}")]
    StdErr {
        #[from]
        source: cosmwasm_std::StdError,
    },
}

/// Percent represents a value between 0 and 100%
/// (i.e. between 0.0 and 1.0)
#[cw_serde]
#[derive(Copy, Default, PartialOrd, Ord, Eq)]
pub struct Percent(#[serde(deserialize_with = "de_decimal_percent")] Decimal);

impl Percent {
    pub fn new(value: Decimal) -> Result<Self, ContractsCommonError> {
        if value > Decimal::one() {
            Err(ContractsCommonError::InvalidPercent(value.to_string()))
        } else {
            Ok(Percent(value))
        }
    }

    pub fn is_zero(&self) -> bool {
        self.0 == Decimal::zero()
    }

    pub fn is_hundred(&self) -> bool {
        self == &Self::hundred()
    }

    pub const fn zero() -> Self {
        Self(Decimal::zero())
    }

    pub const fn hundred() -> Self {
        Self(Decimal::one())
    }

    pub fn from_percentage_value(value: u64) -> Result<Self, ContractsCommonError> {
        Percent::new(Decimal::percent(value))
    }

    pub fn value(&self) -> Decimal {
        self.0
    }

    pub fn round_to_integer(&self) -> u8 {
        let hundred = Decimal::from_ratio(100u32, 1u32);
        // we know the cast from u128 to u8 is a safe one since the internal value must be within 0 - 1 range
        truncate_decimal(hundred * self.0).u128() as u8
    }

    pub fn checked_pow(&self, exp: u32) -> Result<Self, OverflowError> {
        self.0.checked_pow(exp).map(Percent)
    }

    // truncate provided percent to only have 2 decimal places,
    // e.g. convert "0.1234567" into "0.12"
    // the purpose of it is to reduce storage space, in particular for the performance contract
    // since that extra precision gains us nothing
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn round_to_two_decimal_places(&self) -> Self {
        let raw = self.0;

        const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000_000_000_000u128); // 1*10**18
        const THRESHOLD: Decimal = Decimal::permille(5); // 0.005

        // in case it ever changes since it's not exposed in the public API
        debug_assert_eq!(
            DECIMAL_FRACTIONAL,
            Uint128::new(10).pow(Decimal::DECIMAL_PLACES)
        );

        let int = (raw.atomics() * Uint128::new(100)) / DECIMAL_FRACTIONAL;

        #[allow(clippy::unwrap_used)]
        let floored = Decimal::from_atomics(int, 2).unwrap();
        let diff = raw - floored;
        let rounded = if diff >= THRESHOLD {
            // ceil
            floored + Decimal::percent(1)
        } else {
            floored
        };
        Percent(rounded)
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn average(&self, other: &Self) -> Self {
        let sum = self.0 + other.0;
        let inner = Decimal::from_ratio(sum.numerator(), sum.denominator() * Uint128::new(2));
        Percent(inner)
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let adjusted = Decimal::from_ratio(100u32, 1u32) * self.0;
        write!(f, "{adjusted}%")
    }
}

impl FromStr for Percent {
    type Err = ContractsCommonError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Percent::new(Decimal::from_str(s)?)
    }
}

impl Mul<Decimal> for Percent {
    type Output = Decimal;

    fn mul(self, rhs: Decimal) -> Self::Output {
        self.0 * rhs
    }
}

impl Mul<Percent> for Decimal {
    type Output = Decimal;

    fn mul(self, rhs: Percent) -> Self::Output {
        rhs * self
    }
}

impl Fraction<Uint128> for Percent {
    fn numerator(&self) -> Uint128 {
        self.0.numerator()
    }

    fn denominator(&self) -> Uint128 {
        self.0.denominator()
    }

    fn inv(&self) -> Option<Self> {
        Percent::new(self.0.inv()?).ok()
    }
}

impl Deref for Percent {
    type Target = Decimal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// this is not implemented via From traits due to its naive nature and loss of precision
#[cfg(feature = "naive_float")]
pub trait NaiveFloat {
    fn naive_to_f64(&self) -> f64;

    fn naive_try_from_f64(val: f64) -> Result<Self, ContractsCommonError>
    where
        Self: Sized;
}

#[cfg(feature = "naive_float")]
impl NaiveFloat for Decimal {
    fn naive_to_f64(&self) -> f64 {
        use cosmwasm_std::Fraction;

        // note: this conversion loses precision with too many decimal places,
        // but for the purposes of displaying basic performance, that's not an issue
        self.numerator().u128() as f64 / self.denominator().u128() as f64
    }

    fn naive_try_from_f64(val: f64) -> Result<Self, ContractsCommonError>
    where
        Self: Sized,
    {
        // we are only interested in positive values between 0 and 1
        if !(0. ..=1.).contains(&val) {
            return Err(ContractsCommonError::InvalidPercent(val.to_string()));
        }

        fn gcd(mut x: u64, mut y: u64) -> u64 {
            while y > 0 {
                let rem = x % y;
                x = y;
                y = rem;
            }

            x
        }

        fn to_rational(x: f64) -> (u64, u64) {
            let log = x.log2().floor();
            if log >= 0.0 {
                (x as u64, 1)
            } else {
                let num: u64 = (x / f64::EPSILON) as _;
                let den: u64 = (1.0 / f64::EPSILON) as _;
                let gcd = gcd(num, den);
                (num / gcd, den / gcd)
            }
        }

        let (n, d) = to_rational(val);
        Ok(Decimal::from_ratio(n, d))
    }
}

#[cfg(feature = "naive_float")]
impl NaiveFloat for Percent {
    fn naive_to_f64(&self) -> f64 {
        self.0.naive_to_f64()
    }

    fn naive_try_from_f64(val: f64) -> Result<Self, ContractsCommonError>
    where
        Self: Sized,
    {
        Percent::new(Decimal::naive_try_from_f64(val)?)
    }
}

// implement custom Deserialize because we want to validate Percent has the correct range
fn de_decimal_percent<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Decimal::deserialize(deserializer)?;
    if v > Decimal::one() {
        Err(D::Error::custom(
            "provided decimal percent is larger than 100%",
        ))
    } else {
        Ok(v)
    }
}

fn default_unknown() -> String {
    "unknown".to_string()
}

// TODO: there's no reason this couldn't be used for proper binaries, but in that case
// perhaps the struct should get renamed and moved to a "more" common crate
#[cw_serde]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ContractBuildInformation {
    /// Provides the name of the binary, i.e. the content of `CARGO_PKG_NAME` environmental variable.
    #[serde(default = "default_unknown")]
    pub contract_name: String,

    // VERGEN_BUILD_TIMESTAMP
    /// Provides the build timestamp, for example `2021-02-23T20:14:46.558472672+00:00`.
    pub build_timestamp: String,

    // VERGEN_BUILD_SEMVER
    /// Provides the build version, for example `0.1.0-9-g46f83e1`.
    pub build_version: String,

    // VERGEN_GIT_SHA
    /// Provides the hash of the commit that was used for the build, for example `46f83e112520533338245862d366f6a02cef07d4`.
    pub commit_sha: String,

    // VERGEN_GIT_COMMIT_TIMESTAMP
    /// Provides the timestamp of the commit that was used for the build, for example `2021-02-23T08:08:02-05:00`.
    pub commit_timestamp: String,

    // VERGEN_GIT_BRANCH
    /// Provides the name of the git branch that was used for the build, for example `master`.
    pub commit_branch: String,

    // VERGEN_RUSTC_SEMVER
    /// Provides the rustc version that was used for the build, for example `1.52.0-nightly`.
    pub rustc_version: String,

    // VERGEN_CARGO_DEBUG
    /// Provides the cargo debug mode that was used for the build.
    #[serde(default = "default_unknown")]
    pub cargo_debug: String,

    // VERGEN_CARGO_OPT_LEVEL
    /// Provides the opt value set by cargo during the build
    #[serde(default = "default_unknown")]
    pub cargo_opt_level: String,
}

impl ContractBuildInformation {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        ContractBuildInformation {
            contract_name: name.into(),
            build_version: version.into(),
            build_timestamp: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
            commit_sha: option_env!("VERGEN_GIT_SHA")
                .unwrap_or("UNKNOWN")
                .to_string(),
            commit_timestamp: option_env!("VERGEN_GIT_COMMIT_TIMESTAMP")
                .unwrap_or("UNKNOWN")
                .to_string(),
            commit_branch: option_env!("VERGEN_GIT_BRANCH")
                .unwrap_or("UNKNOWN")
                .to_string(),
            rustc_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
            cargo_debug: env!("VERGEN_CARGO_DEBUG").to_string(),
            cargo_opt_level: env!("VERGEN_CARGO_OPT_LEVEL").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_serde() {
        let valid_value = Percent::from_percentage_value(80).unwrap();
        let serialized = serde_json::to_string(&valid_value).unwrap();

        let deserialized: Percent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(valid_value, deserialized);

        let invalid_values = vec!["\"42\"", "\"1.1\"", "\"1.00000001\"", "\"foomp\"", "\"1a\""];
        for invalid_value in invalid_values {
            assert!(serde_json::from_str::<'_, Percent>(invalid_value).is_err())
        }
        assert_eq!(
            serde_json::from_str::<'_, Percent>("\"0.95\"").unwrap(),
            Percent::from_percentage_value(95).unwrap()
        )
    }

    #[test]
    fn percent_to_absolute_integer() {
        let p = serde_json::from_str::<'_, Percent>("\"0.0001\"").unwrap();
        assert_eq!(p.round_to_integer(), 0);

        let p = serde_json::from_str::<'_, Percent>("\"0.0099\"").unwrap();
        assert_eq!(p.round_to_integer(), 0);

        let p = serde_json::from_str::<'_, Percent>("\"0.0199\"").unwrap();
        assert_eq!(p.round_to_integer(), 1);

        let p = serde_json::from_str::<'_, Percent>("\"0.45123\"").unwrap();
        assert_eq!(p.round_to_integer(), 45);

        let p = serde_json::from_str::<'_, Percent>("\"0.999999999\"").unwrap();
        assert_eq!(p.round_to_integer(), 99);

        let p = serde_json::from_str::<'_, Percent>("\"1.00\"").unwrap();
        assert_eq!(p.round_to_integer(), 100);
    }

    #[test]
    #[cfg(feature = "naive_float")]
    fn naive_float_conversion() {
        // around 15 decimal places is the maximum precision we can handle
        // which is still way more than enough for what we use it for
        let float: f64 = "0.546295475423853".parse().unwrap();
        let percent: Percent = "0.546295475423853".parse().unwrap();

        assert_eq!(float, percent.naive_to_f64());

        let epsilon = Decimal::from_ratio(1u64, 1000000000000000u64);
        let converted = Percent::naive_try_from_f64(float).unwrap();

        assert!(converted.0 - converted.0 < epsilon);
    }

    #[test]
    fn rounding_percent() {
        let test_cases = vec![
            ("0", "0"),
            ("0.1", "0.1"),
            ("0.12", "0.12"),
            ("0.12", "0.123"),
            ("0.12", "0.123456789"),
            ("0.13", "0.125"),
            ("0.13", "0.126"),
            ("0.13", "0.126436545676"),
            ("0.99", "0.99"),
            ("0.99", "0.994"),
            ("1", "0.999"),
            ("1", "0.995"),
        ];
        for (expected, input) in test_cases {
            let expected: Percent = expected.parse().unwrap();
            let pre_truncated: Percent = input.parse().unwrap();
            assert_eq!(expected, pre_truncated.round_to_two_decimal_places())
        }
    }

    #[test]
    fn calculating_average() -> anyhow::Result<()> {
        fn p(raw: &str) -> Percent {
            raw.parse().unwrap()
        }

        assert_eq!(p("0.1").average(&p("0.1")), p("0.1"));
        assert_eq!(p("0.1").average(&p("0.2")), p("0.15"));
        assert_eq!(p("1").average(&p("0")), p("0.5"));
        assert_eq!(p("0.123").average(&p("0.456")), p("0.2895"));

        Ok(())
    }
}
