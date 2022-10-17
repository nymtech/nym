// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Decimal;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{self, Display, Formatter};
use std::ops::Mul;
use std::str::FromStr;
use thiserror::Error;

pub fn truncate_decimal(amount: Decimal) -> Uint128 {
    amount * Uint128::new(1)
}

#[derive(Error, Debug)]
pub enum ContractsCommonError {
    #[error("Provided percent value ({0}) is greater than 100%")]
    InvalidPercent(Decimal),

    #[error("{source}")]
    StdErr {
        #[from]
        source: cosmwasm_std::StdError,
    },
}

/// Percent represents a value between 0 and 100%
/// (i.e. between 0.0 and 1.0)
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
pub struct Percent(#[serde(deserialize_with = "de_decimal_percent")] Decimal);

impl Percent {
    pub fn new(value: Decimal) -> Result<Self, ContractsCommonError> {
        if value > Decimal::one() {
            Err(ContractsCommonError::InvalidPercent(value))
        } else {
            Ok(Percent(value))
        }
    }

    pub fn is_zero(&self) -> bool {
        self.0 == Decimal::zero()
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
}

impl Display for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let adjusted = Decimal::from_atomics(100u32, 0).unwrap() * self.0;
        write!(f, "{}%", adjusted)
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

impl Mul<Uint128> for Percent {
    type Output = Uint128;

    fn mul(self, rhs: Uint128) -> Self::Output {
        self.0 * rhs
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

// TODO: there's no reason this couldn't be used for proper binaries, but in that case
// perhaps the struct should get renamed and moved to a "more" common crate
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractBuildInformation {
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
}
