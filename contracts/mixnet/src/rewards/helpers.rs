use crate::error::ContractError;
use cosmwasm_std::Decimal;
use cosmwasm_std::Uint128;
use std::ops::Sub;

// for time being completely ignore concept of a leap year and assume each year is exactly 365 days
// i.e. 8760 hours
const HOURS_IN_YEAR: u128 = 8760;

pub const DECIMAL_FRACTIONAL: Uint128 = Uint128(1_000_000_000_000_000_000u128);

pub(crate) fn calculate_epoch_reward_rate(
    epoch_length: u32,
    annual_reward_rate: Decimal,
) -> Decimal {
    // this is more of a sanity check as the contract does not allow setting annual reward rates
    // to be lower than 1.
    debug_assert!(annual_reward_rate >= Decimal::one());

    // converts reward rate, like 1.25 into the expected gain, like 0.25
    let annual_reward = annual_reward_rate.sub(Decimal::one());
    // do a simple cross-multiplication:
    // `annual_reward`  -    `HOURS_IN_YEAR`
    //          x       -    `epoch_length`
    //
    // x = `annual_reward` * `epoch_length` / `HOURS_IN_YEAR`

    // converts reward, like 0.25 into 250000000000000000
    let annual_reward_uint128 = decimal_to_uint128(annual_reward);

    // calculates `annual_reward_uint128` * `epoch_length` / `HOURS_IN_YEAR`
    let epoch_reward_uint128 = annual_reward_uint128.multiply_ratio(epoch_length, HOURS_IN_YEAR);

    // note: this returns a % reward, like 0.05 rather than reward rate (like 1.05)
    uint128_to_decimal(epoch_reward_uint128)
}

pub(crate) fn scale_reward_by_uptime(
    reward: Decimal,
    uptime: u32,
) -> Result<Decimal, ContractError> {
    if uptime > 100 {
        return Err(ContractError::UnexpectedUptime);
    }
    let uptime_ratio = Decimal::from_ratio(uptime, 100u128);
    // if we do not convert into a more precise representation, we might end up with, for example,
    // reward 0.05 and uptime of 50% which would produce 0.50 * 0.05 = 0 (because of u128 representation)
    // and also the above would be impossible to compute as Mul<Decimal> for Decimal is not implemented
    //
    // but with the intermediate conversion, we would have
    // 0.50 * 50_000_000_000_000_000 = 25_000_000_000_000_000
    // which converted back would give us the proper 0.025
    let uptime_ratio_u128 = decimal_to_uint128(uptime_ratio);
    let scaled = reward * uptime_ratio_u128;
    Ok(uint128_to_decimal(scaled))
}

fn decimal_to_uint128(value: Decimal) -> Uint128 {
    value * DECIMAL_FRACTIONAL
}

fn uint128_to_decimal(value: Uint128) -> Decimal {
    Decimal::from_ratio(value, DECIMAL_FRACTIONAL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn calculating_epoch_reward_rate() {
        // 1.10
        let annual_reward_rate = Decimal::from_ratio(110u128, 100u128);

        // if the epoch is (for some reason) exactly one year,
        // the reward rate should be unchanged
        let per_epoch_rate = calculate_epoch_reward_rate(HOURS_IN_YEAR as u32, annual_reward_rate);
        // 0.10
        let expected = annual_reward_rate.sub(Decimal::one());
        assert_eq!(expected, per_epoch_rate);

        // 24 hours
        let per_epoch_rate = calculate_epoch_reward_rate(24, annual_reward_rate);
        // 0.1 / 365
        let expected = Decimal::from_ratio(1u128, 3650u128);
        assert_eq!(expected, per_epoch_rate);

        let expected_per_epoch_rate_excel = Decimal::from_str("0.000273972602739726").unwrap();
        assert_eq!(expected_per_epoch_rate_excel, per_epoch_rate);

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
