// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};

pub type GranterAddress = Addr;
pub type GranteeAddress = Addr;

pub use grants::*;
pub use query_responses::*;

#[cw_serde]
pub struct TransferRecipient {
    pub recipient: String,
    pub amount: Coin,
}

pub mod grants {
    use crate::utils::ensure_unix_timestamp_not_in_the_past;
    use crate::{GranteeAddress, GranterAddress, NymPoolContractError};
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Coin, Env, Timestamp};

    #[cw_serde]
    pub struct GranterInformation {
        // realistically this is always going to be the contract admin,
        // but let's keep this metadata regardless just in case it ever changes,
        // such as we create a granter controlled by validator multisig or governance
        pub created_by: Addr,
        pub created_at_height: u64,
    }

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
        pub fn expired(&self, env: &Env) -> bool {
            let Some(expiration) = self.basic().expiration_unix_timestamp else {
                return false;
            };
            let current_unix_timestamp = env.block.time.seconds();
            expiration < current_unix_timestamp
        }

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

        /// Perform validation of a new grant that's to be created
        pub fn validate_new(&self, env: &Env, denom: &str) -> Result<(), NymPoolContractError> {
            // 1. perform validation on the inner, basic, allowance
            self.basic().validate(env, denom)?;

            // 2. perform additional validation specific to each variant
            match self {
                // we already validated basic allowance
                Allowance::Basic(_) => Ok(()),
                Allowance::ClassicPeriodic(allowance) => allowance.validate_new_inner(denom),
                Allowance::CumulativePeriodic(allowance) => allowance.validate_new_inner(denom),
                Allowance::Delayed(allowance) => allowance.validate_new_inner(env),
            }
        }

        /// Updates initial state of this allowance settings things such as period reset timestamps.
        pub fn set_initial_state(&mut self, env: &Env) {
            match self {
                // nothing to do for the basic allowance
                Allowance::Basic(_) => {}
                Allowance::ClassicPeriodic(allowance) => allowance.set_initial_state(env),
                Allowance::CumulativePeriodic(allowance) => allowance.set_initial_state(env),
                Allowance::Delayed(allowance) => allowance.set_initial_state(env),
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

        pub(super) fn set_initial_state(&self, env: &Env) {
            todo!()
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
        pub period_spend_limit: Coin,

        /// period_can_spend is the number of coins left to be spent before the period_reset time
        // set by the contract during initialisation of the grant
        #[serde(default)]
        pub period_can_spend: Option<Coin>,

        /// period_reset is the time at which this period resets and a new one begins,
        /// it is calculated from the start time of the first transaction after the
        /// last period ended
        // set by the contract during initialisation of the grant
        #[serde(default)]
        pub period_reset_unix_timestamp: u64,
    }

    impl ClassicPeriodicAllowance {
        pub(super) fn validate_new_inner(&self, denom: &str) -> Result<(), NymPoolContractError> {
            // period duration shouldn't be zero
            if self.period_duration_secs == 0 {
                return Err(NymPoolContractError::ZeroAllowancePeriod);
            }

            // the denom for period spend limit must match the expected value
            if self.period_spend_limit.denom != denom {
                return Err(NymPoolContractError::InvalidDenom {
                    expected: denom.to_string(),
                    got: self.period_spend_limit.denom.to_string(),
                });
            }

            // if the basic spend limit is set, the period spend limit cannot be larger than it
            if let Some(ref basic_limit) = self.basic.spend_limit {
                if basic_limit.amount < self.period_spend_limit.amount {
                    return Err(NymPoolContractError::PeriodicGrantOverSpendLimit {
                        periodic: self.period_spend_limit.clone(),
                        total_limit: basic_limit.clone(),
                    });
                }
            }

            Ok(())
        }

        pub(super) fn set_initial_state(&self, env: &Env) {
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

        /// accumulation_limit is the maximum value the grants and accumulate to
        pub accumulation_limit: Option<Coin>,

        /// spendable is the number of coins left to be spent before additional grant is applied
        // set by the contract during initialisation of the grant
        #[serde(default)]
        pub spendable: Option<Coin>,

        /// last_grant_applied is the time at which last transaction associated with this allowance
        /// has been sent and `spendable` value has been adjusted
        // set by the contract during initialisation of the grant
        #[serde(default)]
        pub last_grant_applied_unix_timestamp: u64,
    }

    impl CumulativePeriodicAllowance {
        pub(super) fn validate_new_inner(&self, denom: &str) -> Result<(), NymPoolContractError> {
            // period duration shouldn't be zero
            if self.period_duration_secs == 0 {
                return Err(NymPoolContractError::ZeroAllowancePeriod);
            }

            // the denom for period grant must match the expected value
            if self.period_grant.denom != denom {
                return Err(NymPoolContractError::InvalidDenom {
                    expected: denom.to_string(),
                    got: self.period_grant.denom.to_string(),
                });
            }

            // the period grant must not be larger than the total spend limit, if set
            if let Some(ref basic_limit) = self.basic.spend_limit {
                if basic_limit.amount < self.period_grant.amount {
                    return Err(NymPoolContractError::PeriodicGrantOverSpendLimit {
                        periodic: self.period_grant.clone(),
                        total_limit: basic_limit.clone(),
                    });
                }
            }

            if let Some(ref accumulation_limit) = self.accumulation_limit {
                // if set, the accumulation limit must not be smaller than the period grant
                if accumulation_limit.amount < self.period_grant.amount {
                    return Err(NymPoolContractError::AccumulationBelowGrantAmount {
                        accumulation: accumulation_limit.clone(),
                        periodic_grant: self.period_grant.clone(),
                    });
                }

                // if set, the denom for accumulation limit must match the expected value
                if accumulation_limit.denom != denom {
                    return Err(NymPoolContractError::InvalidDenom {
                        expected: denom.to_string(),
                        got: accumulation_limit.denom.to_string(),
                    });
                }

                // if set, the accumulation limit must not be larger than the total spend limit
                if let Some(ref basic_limit) = self.basic.spend_limit {
                    if basic_limit.amount < accumulation_limit.amount {
                        return Err(NymPoolContractError::AccumulationOverSpendLimit {
                            accumulation: accumulation_limit.clone(),
                            total_limit: basic_limit.clone(),
                        });
                    }
                }
            }

            Ok(())
        }

        pub(super) fn set_initial_state(&self, env: &Env) {
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
        pub(super) fn validate_new_inner(&self, env: &Env) -> Result<(), NymPoolContractError> {
            // available at must be set in the future
            ensure_unix_timestamp_not_in_the_past(self.available_at_unix_timestamp, env)?;

            // and it must become available before the underlying allowance expires
            if let Some(expiration) = self.basic.expiration_unix_timestamp {
                if expiration < self.available_at_unix_timestamp {
                    return Err(NymPoolContractError::UnattainableDelayedAllowance {
                        expiration_timestamp: expiration,
                        available_timestamp: self.available_at_unix_timestamp,
                    });
                }
            }

            Ok(())
        }

        pub(super) fn set_initial_state(&self, env: &Env) {
            todo!()
        }
    }
}

pub mod query_responses {
    use crate::{Grant, GranteeAddress, GranterAddress, GranterInformation};
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
        pub grantee: GranteeAddress,

        // a `None` value implies no grant
        pub locked: Option<Coin>,
    }

    #[cw_serde]
    pub struct GrantInformation {
        pub grant: Grant,
        pub expired: bool,
    }

    #[cw_serde]
    pub struct GrantResponse {
        pub grantee: GranteeAddress,
        pub grant: Option<GrantInformation>,
    }

    #[cw_serde]
    pub struct GranterResponse {
        pub granter: GranterAddress,
        pub information: Option<GranterInformation>,
    }

    #[cw_serde]
    pub struct GrantsPagedResponse {
        pub grants: Vec<GrantInformation>,
        pub start_next_after: Option<String>,
    }

    #[cw_serde]
    pub struct GranterDetails {
        pub granter: GranterAddress,
        pub information: GranterInformation,
    }

    impl From<(GranterAddress, GranterInformation)> for GranterDetails {
        fn from((granter, information): (GranterAddress, GranterInformation)) -> Self {
            GranterDetails {
                granter,
                information,
            }
        }
    }

    #[cw_serde]
    pub struct GrantersPagedResponse {
        pub granters: Vec<GranterDetails>,
        pub start_next_after: Option<String>,
    }

    #[cw_serde]
    pub struct LockedTokens {
        pub grantee: GranteeAddress,
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
            spend_limit: Some(coin(100000, TEST_DENOM)),
            expiration_unix_timestamp: Some(1643652000),
        }
    }

    fn mock_classic_periodic_allowance() -> ClassicPeriodicAllowance {
        ClassicPeriodicAllowance {
            basic: mock_basic_allowance(),
            period_duration_secs: 10,
            period_spend_limit: coin(1000, TEST_DENOM),
            period_can_spend: None,
            period_reset_unix_timestamp: 0,
        }
    }

    fn mock_cumulative_periodic_allowance() -> CumulativePeriodicAllowance {
        CumulativePeriodicAllowance {
            basic: mock_basic_allowance(),
            period_duration_secs: 10,
            period_grant: coin(1000, TEST_DENOM),
            accumulation_limit: Some(coin(10000, TEST_DENOM)),
            spendable: None,
            last_grant_applied_unix_timestamp: 0,
        }
    }

    fn mock_delayed_allowance() -> DelayedAllowance {
        DelayedAllowance {
            basic: mock_basic_allowance(),
            available_at_unix_timestamp: 1643650000,
        }
    }

    #[cfg(test)]
    mod validating_new_allowances {
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
            use crate::NymPoolContractError;

            #[test]
            fn period_duration_must_be_nonzero() {
                let mut allowance = mock_classic_periodic_allowance();

                allowance.period_duration_secs = 0;
                assert_eq!(
                    allowance.validate_new_inner(TEST_DENOM).unwrap_err(),
                    NymPoolContractError::ZeroAllowancePeriod
                );

                allowance.period_duration_secs = 1;
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }

            #[test]
            fn spend_limit_must_match_expected_denom() {
                let allowance = mock_classic_periodic_allowance();

                // mismatched denom
                assert!(allowance.validate_new_inner("baddenom").is_err());

                // matched denom
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }

            #[test]
            fn period_spend_limit_must_be_smaller_than_total_limit() {
                let mut allowance = mock_classic_periodic_allowance();

                let total_limit = coin(1000, TEST_DENOM);
                allowance.basic.spend_limit = Some(total_limit);

                // above total spend limit
                allowance.period_spend_limit = coin(1001, TEST_DENOM);
                assert!(matches!(
                    allowance.validate_new_inner(TEST_DENOM).unwrap_err(),
                    NymPoolContractError::PeriodicGrantOverSpendLimit { .. }
                ));

                // below total spend limit
                allowance.period_spend_limit = coin(999, TEST_DENOM);
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // equal to total spend limit
                allowance.period_spend_limit = coin(1000, TEST_DENOM);
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // no total spend limit
                allowance.basic.spend_limit = None;
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }
        }

        #[cfg(test)]
        mod cumulative_periodic_allowance {
            use super::*;
            use crate::NymPoolContractError;

            #[test]
            fn period_duration_must_be_nonzero() {
                let mut allowance = mock_cumulative_periodic_allowance();

                allowance.period_duration_secs = 0;
                assert_eq!(
                    allowance.validate_new_inner(TEST_DENOM).unwrap_err(),
                    NymPoolContractError::ZeroAllowancePeriod
                );

                allowance.period_duration_secs = 1;
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }

            #[test]
            fn grant_must_match_expected_denom() {
                let allowance = mock_cumulative_periodic_allowance();

                // mismatched denom
                assert!(allowance.validate_new_inner("baddenom").is_err());

                // matched denom
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }

            #[test]
            fn grant_amount_must_be_smaller_than_total_limit() {
                let mut allowance = mock_cumulative_periodic_allowance();

                let total_limit = coin(1000, TEST_DENOM);
                allowance.basic.spend_limit = Some(total_limit);
                allowance.accumulation_limit = None;

                // above total spend limit
                allowance.period_grant = coin(1001, TEST_DENOM);
                assert!(matches!(
                    allowance.validate_new_inner(TEST_DENOM).unwrap_err(),
                    NymPoolContractError::PeriodicGrantOverSpendLimit { .. }
                ));

                // below total spend limit
                allowance.period_grant = coin(999, TEST_DENOM);
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // equal to total spend limit
                allowance.period_grant = coin(1000, TEST_DENOM);
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // no total spend limit
                allowance.basic.spend_limit = None;
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }

            #[test]
            fn accumulation_limit_must_be_smaller_than_total_limit() {
                let mut allowance = mock_cumulative_periodic_allowance();

                let total_limit = coin(1000, TEST_DENOM);
                allowance.basic.spend_limit = Some(total_limit.clone());
                allowance.period_grant = coin(500, TEST_DENOM);

                // above total spend limit
                allowance.accumulation_limit = Some(coin(1001, TEST_DENOM));
                assert!(matches!(
                    allowance.validate_new_inner(TEST_DENOM).unwrap_err(),
                    NymPoolContractError::AccumulationOverSpendLimit { .. }
                ));

                // below total spend limit
                allowance.accumulation_limit = Some(coin(999, TEST_DENOM));
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // equal to total spend limit
                allowance.accumulation_limit = Some(coin(1000, TEST_DENOM));
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // no total spend limit
                allowance.basic.spend_limit = None;
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // no accumulation limit
                allowance.accumulation_limit = None;
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // no accumulation limit but with spend limit
                allowance.basic.spend_limit = Some(total_limit);
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }

            #[test]
            fn accumulation_limit_must_not_be_smaller_than_grant_amount() {
                let mut allowance = mock_cumulative_periodic_allowance();

                let total_limit = coin(1000, TEST_DENOM);
                allowance.basic.spend_limit = Some(total_limit);
                allowance.period_grant = coin(500, TEST_DENOM);

                // below grant amount
                allowance.accumulation_limit = Some(coin(499, TEST_DENOM));
                assert!(matches!(
                    allowance.validate_new_inner(TEST_DENOM).unwrap_err(),
                    NymPoolContractError::AccumulationBelowGrantAmount { .. }
                ));

                // above grant amount
                allowance.accumulation_limit = Some(coin(501, TEST_DENOM));
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // equal to grant amount
                allowance.accumulation_limit = Some(coin(500, TEST_DENOM));
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());

                // no accumulation limit
                allowance.accumulation_limit = None;
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }

            #[test]
            fn accumulation_limit_must_match_expected_denom() {
                let mut allowance = mock_cumulative_periodic_allowance();
                allowance.accumulation_limit = Some(coin(1000, "baddenom"));

                // mismatched denom
                assert!(allowance.validate_new_inner(TEST_DENOM).is_err());

                // matched denom
                allowance.accumulation_limit = Some(coin(1000, TEST_DENOM));
                assert!(allowance.validate_new_inner(TEST_DENOM).is_ok());
            }
        }

        #[cfg(test)]
        mod delayed_allowance {
            use super::*;
            use cosmwasm_std::testing::mock_env;
            use cosmwasm_std::Timestamp;

            #[test]
            fn doesnt_allow_availability_in_the_past() {
                let allowance = mock_delayed_allowance();
                let mut env = mock_env();

                // availability is in the past
                env.block.time = Timestamp::from_seconds(allowance.available_at_unix_timestamp + 1);
                assert!(allowance.validate_new_inner(&env).is_err());

                // availability is equal to the current block time
                env.block.time = Timestamp::from_seconds(allowance.available_at_unix_timestamp);
                assert!(allowance.validate_new_inner(&env).is_ok());

                // availability is in the future
                env.block.time = Timestamp::from_seconds(allowance.available_at_unix_timestamp - 1);
                assert!(allowance.validate_new_inner(&env).is_ok());
            }

            #[test]
            fn must_have_available_before_allowance_expiration() {
                let mut allowance = mock_delayed_allowance();
                let mut env = mock_env();
                env.block.time = Timestamp::from_seconds(100);
                allowance.basic.expiration_unix_timestamp = Some(1000);

                // after expiration
                allowance.available_at_unix_timestamp = 1001;
                assert!(allowance.validate_new_inner(&env).is_err());

                // equal to expiration
                allowance.available_at_unix_timestamp = 1000;
                assert!(allowance.validate_new_inner(&env).is_ok());

                // before expiration
                allowance.available_at_unix_timestamp = 999;
                assert!(allowance.validate_new_inner(&env).is_ok());

                // with no explicit expiration
                allowance.basic.expiration_unix_timestamp = None;
                assert!(allowance.validate_new_inner(&env).is_ok());
            }
        }
    }

    #[cfg(test)]
    mod setting_initial_state {
        use super::*;

        #[test]
        fn basic_allowance() {
            todo!()
        }

        #[test]
        fn classic_periodic_allowance() {
            todo!()
        }

        #[test]
        fn cumulative_periodic_allowance() {
            todo!()
        }

        #[test]
        fn delayed_allowance() {
            todo!()
        }
    }
}
