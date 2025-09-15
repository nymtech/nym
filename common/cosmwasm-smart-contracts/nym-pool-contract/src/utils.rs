// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymPoolContractError;
use cosmwasm_std::Env;

pub fn ensure_unix_timestamp_not_in_the_past(
    unix_timestamp: u64,
    env: &Env,
) -> Result<(), NymPoolContractError> {
    if unix_timestamp < env.block.time.seconds() {
        return Err(NymPoolContractError::TimestampInThePast {
            timestamp: unix_timestamp,
            current_block_timestamp: env.block.time.seconds(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Timestamp;
    use time::macros::datetime;

    #[test]
    fn ensuring_unix_timestamp_not_in_the_past() {
        let unix_epoch = 0;

        let date_in_the_past = datetime!(1984-01-02 3:45 UTC);
        let sane_block_time = datetime!(2025-01-28 12:15 UTC);

        let before_block = datetime!(2025-01-28 12:00 UTC);
        let after_block = datetime!(2025-01-28 12:30 UTC);

        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(sane_block_time.unix_timestamp() as u64);

        let res = ensure_unix_timestamp_not_in_the_past(unix_epoch, &env).unwrap_err();
        assert_eq!(
            NymPoolContractError::TimestampInThePast {
                timestamp: unix_epoch,
                current_block_timestamp: env.block.time.seconds(),
            },
            res
        );

        let res =
            ensure_unix_timestamp_not_in_the_past(date_in_the_past.unix_timestamp() as u64, &env)
                .unwrap_err();
        assert_eq!(
            NymPoolContractError::TimestampInThePast {
                timestamp: date_in_the_past.unix_timestamp() as u64,
                current_block_timestamp: env.block.time.seconds(),
            },
            res
        );

        let res = ensure_unix_timestamp_not_in_the_past(before_block.unix_timestamp() as u64, &env)
            .unwrap_err();
        assert_eq!(
            NymPoolContractError::TimestampInThePast {
                timestamp: before_block.unix_timestamp() as u64,
                current_block_timestamp: env.block.time.seconds(),
            },
            res
        );

        let res =
            ensure_unix_timestamp_not_in_the_past(sane_block_time.unix_timestamp() as u64, &env);
        assert!(res.is_ok());

        let res = ensure_unix_timestamp_not_in_the_past(after_block.unix_timestamp() as u64, &env);
        assert!(res.is_ok());
    }
}
