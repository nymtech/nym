// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::error::ContractError;
use cosmwasm_std::{Decimal, Uint128};

// for time being completely ignore concept of a leap year and assume each year is exactly 365 days
// i.e. 8760 hours
const HOURS_IN_YEAR: u128 = 8760;

// annoyingly not exposed by `Decimal` directly.
const DECIMAL_FRACTIONAL: Uint128 = Uint128(1_000_000_000_000_000_000u128);

// calculates value - 1
fn decimal_sub_one(value: Decimal) -> Decimal {
    assert!(value >= Decimal::one());

    // those conversions are so freaking disgusting and I fear they might result in some loss of precision
    let value_uint128 = value * DECIMAL_FRACTIONAL;
    let uint128_sub_one = (value_uint128 - DECIMAL_FRACTIONAL).unwrap();
    Decimal::from_ratio(uint128_sub_one, DECIMAL_FRACTIONAL)
}

// I don't like this, but this seems to be the only way of converting Decimal into Uint128
fn decimal_to_uint128(value: Decimal) -> Uint128 {
    // TODO: This function should have some proper bound checks implemented to ensure no overflow
    value * DECIMAL_FRACTIONAL
}

// another disgusting conversion, assumes `value` was already multiplied by `DECIMAL_FRACTIONAL` before
fn uint128_to_decimal(value: Uint128) -> Decimal {
    Decimal::from_ratio(value, uint128_decimal_one())
}

const fn uint128_decimal_one() -> Uint128 {
    DECIMAL_FRACTIONAL
}

// TODO: this does not seem fully right, I'm not sure what that is exactly,
// but it feels like something is not taken into consideration,
// like compound interest BS or some other exponentiation stuff
pub(crate) fn calculate_epoch_reward_rate(
    epoch_length: u32,
    annual_reward_rate: Decimal,
) -> Decimal {
    // this is more of a sanity check as the contract does not allow setting annual reward rates
    // to be lower than 1.
    debug_assert!(annual_reward_rate >= Decimal::one());

    // converts reward rate, like 1.25 into the expected gain, like 0.25
    let annual_reward = decimal_sub_one(annual_reward_rate);
    // do a simple cross-multiplication:
    // `annual_reward`  -    `HOURS_IN_YEAR`
    //          x       -    `epoch_length`
    //
    // x = `annual_reward` * `epoch_length` / `HOURS_IN_YEAR`

    let epoch_ratio = Decimal::from_ratio(epoch_length, HOURS_IN_YEAR);

    // converts reward, like 0.25 into 250000000000000000
    let annual_reward_uint128 = decimal_to_uint128(annual_reward);

    let epoch_reward_uint128 = epoch_ratio * annual_reward_uint128;

    // note: this returns a % reward, like 0.05 rather than reward rate (like 1.05)
    uint128_to_decimal(epoch_reward_uint128)
}

// this function works under assumption that epoch reward has relatively few decimal places
// (I think, but to be verified, less than 18)
// uptime must be a value in range of 0-100
pub(crate) fn scale_reward_by_uptime(
    reward: Decimal,
    uptime: u32,
) -> Result<Decimal, ContractError> {
    if uptime > 100 {
        return Err(ContractError::UnexpectedUptime);
    }
    let uptime_ratio = Decimal::from_ratio(uptime, 100u128);
    let uptime_ratio_u128 = decimal_to_uint128(uptime_ratio);
    let scaled = reward * uptime_ratio_u128;
    Ok(uint128_to_decimal(scaled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculating_epoch_reward_rate() {
        // 1.10
        let annual_reward_rate = Decimal::from_ratio(110u128, 100u128);
        let annual_reward = decimal_sub_one(annual_reward_rate);

        // if the epoch is (for some reason) exactly one year,
        // the reward rate should be unchanged
        let per_epoch_rate = calculate_epoch_reward_rate(HOURS_IN_YEAR as u32, annual_reward_rate);
        assert_eq!(per_epoch_rate, annual_reward);

        // 24 hours
        let per_epoch_rate = calculate_epoch_reward_rate(24, annual_reward_rate);

        // 0.1 / 365
        let expected = Decimal::from_ratio(1u128, 3650u128);

        assert_eq!(expected, per_epoch_rate);

        // 1 hour
        let per_epoch_rate = calculate_epoch_reward_rate(1, annual_reward_rate);

        // 0.1 / 8760
        let expected = Decimal::from_ratio(1u128, 87600u128);

        assert_eq!(expected, per_epoch_rate);
    }

    #[test]
    fn scaling_reward_by_uptime() {
        // 0.05
        let epoch_reward = Decimal::from_ratio(5u128, 100u128);

        // scaling by 100 does nothing
        let scaled = scale_reward_by_uptime(epoch_reward, 100).unwrap();
        assert_eq!(epoch_reward, scaled);

        // scaling by 0 makes the reward 0
        let scaled = scale_reward_by_uptime(epoch_reward, 0).unwrap();
        assert_eq!(Decimal::zero(), scaled);

        // 50 halves it
        let scaled = scale_reward_by_uptime(epoch_reward, 50).unwrap();
        let expected = Decimal::from_ratio(25u128, 1000u128);
        assert_eq!(expected, scaled);

        // 10 takes 1/10th
        let scaled = scale_reward_by_uptime(epoch_reward, 10).unwrap();
        let expected = Decimal::from_ratio(5u128, 1000u128);
        assert_eq!(expected, scaled);

        // anything larger than 100 returns an error
        assert!(scale_reward_by_uptime(epoch_reward, 101).is_err())
    }
}
