// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;

pub type GranterAddress = Addr;
pub type GranteeAddress = Addr;

pub use grants::*;
pub use query_responses::*;

pub mod grants {
    use crate::utils::ensure_unix_timestamp_not_in_the_past;
    use crate::{GranteeAddress, GranterAddress, NymPoolContractError};
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Coin, Env, Timestamp};

    #[cw_serde]
    pub struct Grant {
        pub granter: GranterAddress,
        pub grantee: GranteeAddress,
        pub granted_at_height: u64,
        pub allowance: Allowance,
    }

    #[cw_serde]
    pub enum Allowance {
        Basic(BasicAllowance),
        ClassicPeriodic(ClassicPeriodicAllowance),
        CumulativePeriodic(CumulativePeriodicAllowance),
        Delayed(DelayedAllowance),
    }

    impl Allowance {
        pub fn basic(&self) -> &BasicAllowance {
            match self {
                Allowance::Basic(allowance) => allowance,
                Allowance::ClassicPeriodic(allowance) => &allowance.basic,
                Allowance::CumulativePeriodic(allowance) => &allowance.basic,
                Allowance::Delayed(allowance) => &allowance.basic,
            }
        }

        pub fn expiration(&self) -> Option<Timestamp> {
            let expiration_unix = match self {
                Allowance::Basic(allowance) => allowance.expiration_unix_timestamp,
                Allowance::ClassicPeriodic(allowance) => allowance.basic.expiration_unix_timestamp,
                Allowance::CumulativePeriodic(allowance) => {
                    allowance.basic.expiration_unix_timestamp
                }
                Allowance::Delayed(allowance) => allowance.basic.expiration_unix_timestamp,
            };

            expiration_unix.map(Timestamp::from_seconds)
        }

        pub fn validate(&self, env: &Env, denom: &str) -> Result<(), NymPoolContractError> {
            // 1. perform validation on the inner, basic, allowance
            self.basic().validate(env, denom)?;

            // 2. perform additional validation specific to each variant
            match self {
                // we already validated basic allowance
                Allowance::Basic(_) => Ok(()),
                Allowance::ClassicPeriodic(allowance) => allowance.validate(env, denom),
                Allowance::CumulativePeriodic(allowance) => allowance.validate(env, denom),
                Allowance::Delayed(allowance) => allowance.validate(env, denom),
            }
        }
    }

    /// BasicAllowance is an allowance with a one-time grant of coins
    /// that optionally expires. The grantee can use up to SpendLimit to cover fees.
    #[cw_serde]
    pub struct BasicAllowance {
        /// spend_limit specifies the maximum amount of coins that can be spent
        /// by this allowance and will be updated as coins are spent. If it is
        /// empty, there is no spend limit and any amount of coins can be spent.
        pub spend_limit: Option<Coin>,

        /// expiration specifies an optional time when this allowance expires
        pub expiration_unix_timestamp: Option<u64>,
    }

    impl BasicAllowance {
        pub fn validate(&self, env: &Env, denom: &str) -> Result<(), NymPoolContractError> {
            // expiration shouldn't be in the past.
            if let Some(expiration) = self.expiration_unix_timestamp {
                ensure_unix_timestamp_not_in_the_past(expiration, env)?;
            }

            // if spend limit is set, it must use the same denomination as the underlying pool
            if let Some(ref spend_limit) = self.spend_limit {
                if spend_limit.denom != denom {
                    return Err(NymPoolContractError::InvalidDenom {
                        expected: denom.to_string(),
                        got: spend_limit.denom.to_string(),
                    });
                }
            }

            Ok(())
        }
    }

    /// ClassicPeriodicAllowance extends BasicAllowance to allow for both a maximum cap,
    /// as well as a limit per time period.
    #[cw_serde]
    pub struct ClassicPeriodicAllowance {
        /// basic specifies a struct of `BasicAllowance`
        pub basic: BasicAllowance,

        /// period_duration_secs specifies the time duration in which period_spend_limit coins can
        /// be spent before that allowance is reset
        pub period_duration_secs: u64,

        /// period_spend_limit specifies the maximum number of coins that can be spent
        /// in the period
        pub period_spend_limit: Option<Coin>,

        /// period_can_spend is the number of coins left to be spent before the period_reset time
        pub period_can_spend: Option<Coin>,

        /// period_reset is the time at which this period resets and a new one begins,
        /// it is calculated from the start time of the first transaction after the
        /// last period ended
        pub period_reset_unix_timestamp: u64,
    }

    impl ClassicPeriodicAllowance {
        pub fn validate(&self, env: &Env, denom: &str) -> Result<(), NymPoolContractError> {
            todo!()
        }
    }

    #[cw_serde]
    pub struct CumulativePeriodicAllowance {
        /// basic specifies a struct of `BasicAllowance`
        pub basic: BasicAllowance,

        /// period_duration_secs specifies the time duration in which spendable coins can
        /// be spent before that allowance is incremented
        pub period_duration_secs: u64,

        /// period_grant specifies the maximum number of coins that is granted per period
        pub period_grant: Coin,

        /// spendable_limit is the maximum value the grants and accumulate to
        pub spendable_limit: Option<Coin>,

        /// spendable is the number of coins left to be spent before additional grant is applied
        pub spendable: Coin,

        /// last_grant_applied is the time at which last transaction associated with this allowance
        /// has been sent and `spendable` value has been adjusted
        pub last_grant_applied_unix_timestamp: u64,
    }

    impl CumulativePeriodicAllowance {
        pub fn validate(&self, env: &Env, denom: &str) -> Result<(), NymPoolContractError> {
            todo!()
        }
    }

    /// Create a grant to allow somebody to withdraw from the pool only after the specified time.
    /// For example, we could create a grant for mixnet rewarding/testing/etc
    /// However, if the required work has not been completed, the grant could be revoked before it's withdrawn
    #[cw_serde]
    pub struct DelayedAllowance {
        /// basic specifies a struct of `BasicAllowance`
        pub basic: BasicAllowance,

        /// available_at specifies when this allowance is going to become usable
        pub available_at_unix_timestamp: u64,
    }

    impl DelayedAllowance {
        pub fn validate(&self, env: &Env, denom: &str) -> Result<(), NymPoolContractError> {
            todo!()
        }
    }
}

pub mod query_responses {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Coin};

    #[cw_serde]
    pub struct AvailableTokensResponse {
        pub available: Coin,
    }

    #[cw_serde]
    pub struct TotalLockedTokensResponse {
        pub locked: Coin,
    }

    #[cw_serde]
    pub struct LockedTokensResponse {
        pub grantee: Addr,

        // a `None` value implies no grant
        pub locked: Option<Coin>,
    }

    #[cw_serde]
    pub struct LockedTokens {
        pub grantee: Addr,
        pub locked: Coin,
    }

    #[cw_serde]
    pub struct LockedTokensPagedResponse {
        pub locked: Vec<LockedTokens>,
        pub start_next_after: Option<String>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::coin;

    const TEST_DENOM: &str = "unym";

    fn mock_basic_allowance() -> BasicAllowance {
        BasicAllowance {
            spend_limit: Some(coin(12345, TEST_DENOM)),
            expiration_unix_timestamp: Some(1643652000),
        }
    }

    #[cfg(test)]
    mod validating_allowances {
        use super::*;

        #[cfg(test)]
        mod basic_allowance {
            use super::*;
            use cosmwasm_std::testing::mock_env;
            use cosmwasm_std::Timestamp;

            #[test]
            fn doesnt_allow_expirations_in_the_past() {
                let mut allowance = mock_basic_allowance();

                let mut env = mock_env();

                // allowance expiration is in the past
                env.block.time =
                    Timestamp::from_seconds(allowance.expiration_unix_timestamp.unwrap() + 1);
                assert!(allowance.validate(&env, TEST_DENOM).is_err());

                // allowance expiration is equal to the current block time
                env.block.time =
                    Timestamp::from_seconds(allowance.expiration_unix_timestamp.unwrap());
                assert!(allowance.validate(&env, TEST_DENOM).is_ok());

                // allowance expiration is in the future
                env.block.time =
                    Timestamp::from_seconds(allowance.expiration_unix_timestamp.unwrap() - 1);
                assert!(allowance.validate(&env, TEST_DENOM).is_ok());

                // no explicit expiration
                allowance.expiration_unix_timestamp = None;
                assert!(allowance.validate(&env, TEST_DENOM).is_ok());
            }

            #[test]
            fn spend_limit_must_match_expected_denom() {
                let mut allowance = mock_basic_allowance();

                let env = mock_env();

                // mismatched denom
                assert!(allowance.validate(&env, "baddenom").is_err());

                // matched denom
                assert!(allowance.validate(&env, TEST_DENOM).is_ok());

                // no spend limit
                allowance.spend_limit = None;
                assert!(allowance.validate(&env, TEST_DENOM).is_ok());
            }
        }

        #[cfg(test)]
        mod classic_periodic_allowance {
            use super::*;
        }

        #[cfg(test)]
        mod cumulative_periodic_allowance {
            use super::*;
        }

        #[cfg(test)]
        mod delayed_allowance {
            use super::*;
        }
    }
}
