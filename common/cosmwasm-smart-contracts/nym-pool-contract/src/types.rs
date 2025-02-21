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
    use cosmwasm_std::{Addr, Coin, Env, Timestamp, Uint128};
    use std::cmp::min;

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

    impl From<BasicAllowance> for Allowance {
        fn from(value: BasicAllowance) -> Self {
            Allowance::Basic(value)
        }
    }

    impl From<ClassicPeriodicAllowance> for Allowance {
        fn from(value: ClassicPeriodicAllowance) -> Self {
            Allowance::ClassicPeriodic(value)
        }
    }

    impl From<CumulativePeriodicAllowance> for Allowance {
        fn from(value: CumulativePeriodicAllowance) -> Self {
            Allowance::CumulativePeriodic(value)
        }
    }

    impl From<DelayedAllowance> for Allowance {
        fn from(value: DelayedAllowance) -> Self {
            Allowance::Delayed(value)
        }
    }

    impl Allowance {
        pub fn expired(&self, env: &Env) -> bool {
            self.basic().expired(env)
        }

        pub fn basic(&self) -> &BasicAllowance {
            match self {
                Allowance::Basic(allowance) => allowance,
                Allowance::ClassicPeriodic(allowance) => &allowance.basic,
                Allowance::CumulativePeriodic(allowance) => &allowance.basic,
                Allowance::Delayed(allowance) => &allowance.basic,
            }
        }

        pub fn basic_mut(&mut self) -> &mut BasicAllowance {
            match self {
                Allowance::Basic(ref mut allowance) => allowance,
                Allowance::ClassicPeriodic(ref mut allowance) => &mut allowance.basic,
                Allowance::CumulativePeriodic(ref mut allowance) => &mut allowance.basic,
                Allowance::Delayed(ref mut allowance) => &mut allowance.basic,
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
                // nothing to do for the delayed allowance
                Allowance::Delayed(_) => {}
            }
        }

        pub fn try_update_state(&mut self, env: &Env) {
            match self {
                // nothing to do for the basic allowance
                Allowance::Basic(_) => {}
                Allowance::ClassicPeriodic(allowance) => allowance.try_update_state(env),
                Allowance::CumulativePeriodic(allowance) => allowance.try_update_state(env),
                // nothing to do for the delayed allowance
                Allowance::Delayed(_) => {}
            }
        }

        pub fn within_spendable_limits(&self, amount: &Coin) -> bool {
            match self {
                Allowance::Basic(allowance) => allowance.within_spendable_limits(amount),
                Allowance::ClassicPeriodic(allowance) => allowance.within_spendable_limits(amount),
                Allowance::CumulativePeriodic(allowance) => {
                    allowance.within_spendable_limits(amount)
                }
                Allowance::Delayed(allowance) => allowance.within_spendable_limits(amount),
            }
        }

        // check whether given the current allowance state, the provided amount could be spent
        // note: it's responsibility of the caller to call `try_update_state` before the call.
        pub fn ensure_can_spend(
            &self,
            env: &Env,
            amount: &Coin,
        ) -> Result<(), NymPoolContractError> {
            match self {
                Allowance::Basic(allowance) => allowance.ensure_can_spend(env, amount),
                Allowance::ClassicPeriodic(allowance) => allowance.ensure_can_spend(env, amount),
                Allowance::CumulativePeriodic(allowance) => allowance.ensure_can_spend(env, amount),
                Allowance::Delayed(allowance) => allowance.ensure_can_spend(env, amount),
            }
        }

        pub fn try_spend(&mut self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            self.try_update_state(env);

            match self {
                Allowance::Basic(allowance) => allowance.try_spend(env, amount),
                Allowance::ClassicPeriodic(allowance) => allowance.try_spend(env, amount),
                Allowance::CumulativePeriodic(allowance) => allowance.try_spend(env, amount),
                Allowance::Delayed(allowance) => allowance.try_spend(env, amount),
            }
        }

        pub fn increase_spend_limit(&mut self, amount: Uint128) {
            if let Some(ref mut limit) = self.basic_mut().spend_limit {
                limit.amount += amount
            }
        }

        pub fn is_used_up(&self) -> bool {
            let Some(ref limit) = self.basic().spend_limit else {
                return false;
            };
            limit.amount.is_zero()
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
        pub fn unlimited() -> BasicAllowance {
            BasicAllowance {
                spend_limit: None,
                expiration_unix_timestamp: None,
            }
        }

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

                if spend_limit.amount.is_zero() {
                    return Err(NymPoolContractError::ZeroAmount);
                }
            }

            Ok(())
        }

        pub fn expired(&self, env: &Env) -> bool {
            let Some(expiration) = self.expiration_unix_timestamp else {
                return false;
            };
            let current_unix_timestamp = env.block.time.seconds();

            expiration < current_unix_timestamp
        }

        fn within_spendable_limits(&self, amount: &Coin) -> bool {
            let Some(ref spend_limit) = self.spend_limit else {
                // if there's no spend limit then whatever the amount is, it's spendable
                return true;
            };

            spend_limit.amount >= amount.amount
        }

        fn ensure_can_spend(&self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            if self.expired(env) {
                return Err(NymPoolContractError::GrantExpired);
            }
            if !self.within_spendable_limits(amount) {
                return Err(NymPoolContractError::SpendingAboveAllowance);
            }
            Ok(())
        }

        fn try_spend(&mut self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            self.ensure_can_spend(env, amount)?;

            if let Some(ref mut spend_limit) = self.spend_limit {
                spend_limit.amount -= amount.amount;
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

            if self.period_spend_limit.amount.is_zero() {
                return Err(NymPoolContractError::ZeroAmount);
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

        /// The value that can be spent in the period is the lesser of the basic spend limit
        /// and the period spend limit
        ///
        /// ```go
        ///    if _, isNeg := a.Basic.SpendLimit.SafeSub(a.PeriodSpendLimit...); isNeg && !a.Basic.SpendLimit.Empty() {
        ///        a.PeriodCanSpend = a.Basic.SpendLimit
        ///    } else {
        ///        a.PeriodCanSpend = a.PeriodSpendLimit
        ///    }
        /// ```
        fn determine_period_can_spend(&self) -> Coin {
            let Some(ref basic_limit) = self.basic.spend_limit else {
                // if there's no spend limit, there's nothing to compare against
                return self.period_spend_limit.clone();
            };

            if basic_limit.amount < self.period_spend_limit.amount {
                basic_limit.clone()
            } else {
                self.period_spend_limit.clone()
            }
        }

        pub(super) fn set_initial_state(&mut self, env: &Env) {
            self.try_update_state(env);
        }

        /// try_update_state will check if the period_reset_unix_timestamp has been hit. If not, it is a no-op.
        /// If we hit the reset period, it will top up the period_can_spend amount to
        /// min(period_spend_limit, basic.spend_limit) so it is never more than the maximum allowed.
        /// It will also update the period_reset_unix_timestamp.
        ///
        /// If we are within one period, it will update from the
        /// last period_reset (eg. if you always do one tx per day, it will always reset the same time)
        /// If we are more than one period out (eg. no activity in a week), reset is one period from the execution of this method
        pub fn try_update_state(&mut self, env: &Env) {
            if env.block.time.seconds() < self.period_reset_unix_timestamp {
                // we haven't yet reached the reset time
                return;
            }
            self.period_can_spend = Some(self.determine_period_can_spend());

            // If we are within the period, step from expiration (eg. if you always do one tx per day,
            // it will always reset the same time)
            // If we are more then one period out (eg. no activity in a week),
            // reset is one period from this time
            self.period_reset_unix_timestamp += self.period_duration_secs;
            if env.block.time.seconds() > self.period_duration_secs {
                self.period_reset_unix_timestamp =
                    env.block.time.seconds() + self.period_duration_secs;
            }
        }

        fn within_spendable_limits(&self, amount: &Coin) -> bool {
            let Some(ref available) = self.period_can_spend else {
                return false;
            };
            available.amount >= amount.amount
        }

        fn ensure_can_spend(&self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            if self.basic.expired(env) {
                return Err(NymPoolContractError::GrantExpired);
            }
            if !self.within_spendable_limits(amount) {
                return Err(NymPoolContractError::SpendingAboveAllowance);
            }
            Ok(())
        }

        fn try_spend(&mut self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            self.ensure_can_spend(env, amount)?;

            // deduct from both the current period and the max amount
            if let Some(ref mut spend_limit) = self.basic.spend_limit {
                spend_limit.amount -= amount.amount;
            }

            // SAFETY: initial `period_can_spend` value is always unconditionally set by the contract during
            // grant creation
            #[allow(clippy::unwrap_used)]
            let period_can_spend = self.period_can_spend.as_mut().unwrap();
            period_can_spend.amount -= amount.amount;

            Ok(())
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

            if self.period_grant.amount.is_zero() {
                return Err(NymPoolContractError::ZeroAmount);
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

        pub(super) fn set_initial_state(&mut self, env: &Env) {
            self.last_grant_applied_unix_timestamp = env.block.time.seconds();

            // initially we can spend equivalent of a single grant
            self.spendable = Some(self.period_grant.clone())
        }

        #[inline]
        fn missed_periods(&self, env: &Env) -> u64 {
            (env.block.time.seconds() - self.last_grant_applied_unix_timestamp)
                % self.period_duration_secs
        }

        /// The value that can be spent is the last of the basic spend limit, the accumulation limit
        /// and number of missed periods multiplied by the period grant
        fn determine_spendable(&self, env: &Env) -> Coin {
            // SAFETY: initial `spendable` value is always unconditionally set by the contract during
            // grant creation
            #[allow(clippy::unwrap_used)]
            let spendable = self.spendable.as_ref().unwrap();

            let missed_periods = self.missed_periods(env);
            let mut max_spendable = spendable.clone();
            max_spendable.amount += Uint128::new(missed_periods as u128) * self.period_grant.amount;

            match (&self.basic.spend_limit, &self.accumulation_limit) {
                (Some(spend_limit), Some(accumulation_limit)) => {
                    let limit = min(spend_limit.amount, accumulation_limit.amount);
                    let amount = min(limit, max_spendable.amount);
                    Coin::new(amount, max_spendable.denom)
                }
                (None, Some(accumulation_limit)) => {
                    let amount = min(accumulation_limit.amount, max_spendable.amount);
                    Coin::new(amount, max_spendable.denom)
                }
                (Some(spend_limit), None) => {
                    let amount = min(spend_limit.amount, max_spendable.amount);
                    Coin::new(amount, max_spendable.denom)
                }
                (None, None) => max_spendable,
            }
        }

        /// try_update_state will check if we've rolled over into the next grant period. If not, it is a no-op.
        /// If we hit the next period, it will top up the spendable amount to
        /// min(accumulation_limit, basic.spend_limit, spendable + period_grant * num_missed_periods) so it is never more than the maximum allowed.
        /// It will also update the last_grant_applied_unix_timestamp.
        pub fn try_update_state(&mut self, env: &Env) {
            let missed_periods = self.missed_periods(env);

            if missed_periods == 0 {
                // we haven't yet reached the next grant time
                return;
            }

            self.spendable = Some(self.determine_spendable(env))
        }

        fn within_spendable_limits(&self, amount: &Coin) -> bool {
            let Some(ref available) = self.spendable else {
                return false;
            };
            available.amount >= amount.amount
        }

        fn ensure_can_spend(&self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            if self.basic.expired(env) {
                return Err(NymPoolContractError::GrantExpired);
            }
            if !self.within_spendable_limits(amount) {
                return Err(NymPoolContractError::SpendingAboveAllowance);
            }
            Ok(())
        }

        fn try_spend(&mut self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            self.ensure_can_spend(env, amount)?;

            // deduct from both the current period and the max amount
            if let Some(ref mut spend_limit) = self.basic.spend_limit {
                spend_limit.amount -= amount.amount;
            }

            // SAFETY: initial `spendable` value is always unconditionally set by the contract during
            // grant creation
            #[allow(clippy::unwrap_used)]
            let spendable = self.spendable.as_mut().unwrap();
            spendable.amount -= amount.amount;

            Ok(())
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

        fn within_spendable_limits(&self, amount: &Coin) -> bool {
            self.basic.within_spendable_limits(amount)
        }

        fn ensure_can_spend(&self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            if self.basic.expired(env) {
                return Err(NymPoolContractError::GrantExpired);
            }
            if !self.within_spendable_limits(amount) {
                return Err(NymPoolContractError::SpendingAboveAllowance);
            }
            if self.available_at_unix_timestamp < env.block.time.seconds() {
                return Err(NymPoolContractError::GrantNotYetAvailable {
                    available_at_timestamp: self.available_at_unix_timestamp,
                });
            }

            Ok(())
        }

        fn try_spend(&mut self, env: &Env, amount: &Coin) -> Result<(), NymPoolContractError> {
            self.ensure_can_spend(env, amount)?;

            if let Some(ref mut spend_limit) = self.basic.spend_limit {
                spend_limit.amount -= amount.amount;
            }

            Ok(())
        }
    }
}

pub mod query_responses {
    use crate::{Grant, GranteeAddress, GranterAddress, GranterInformation};
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::Coin;

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
    use cosmwasm_std::{coin, Uint128};

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

    #[test]
    fn increasing_spend_limit() {
        // no-op if there's no limit
        let mut basic = mock_basic_allowance();
        basic.spend_limit = None;
        let mut basic = Allowance::Basic(basic);

        let mut classic = mock_classic_periodic_allowance();
        classic.basic.spend_limit = None;
        let mut classic = Allowance::ClassicPeriodic(classic);

        let mut cumulative = mock_cumulative_periodic_allowance();
        cumulative.basic.spend_limit = None;
        let mut cumulative = Allowance::CumulativePeriodic(cumulative);

        let mut delayed = mock_delayed_allowance();
        delayed.basic.spend_limit = None;
        let mut delayed = Allowance::Delayed(delayed);

        let basic_og = basic.clone();
        let classic_og = classic.clone();
        let cumulative_og = cumulative.clone();
        let delayed_og = delayed.clone();

        basic.increase_spend_limit(Uint128::new(100));
        classic.increase_spend_limit(Uint128::new(100));
        cumulative.increase_spend_limit(Uint128::new(100));
        delayed.increase_spend_limit(Uint128::new(100));

        assert_eq!(basic, basic_og);
        assert_eq!(classic, classic_og);
        assert_eq!(cumulative, cumulative_og);
        assert_eq!(delayed, delayed_og);

        // adds to spend limit otherwise
        let limit = coin(1000, TEST_DENOM);
        let mut basic = mock_basic_allowance();
        basic.spend_limit = Some(limit.clone());
        let mut basic = Allowance::Basic(basic);

        let mut classic = mock_classic_periodic_allowance();
        classic.basic.spend_limit = Some(limit.clone());
        let mut classic = Allowance::ClassicPeriodic(classic);

        let mut cumulative = mock_cumulative_periodic_allowance();
        cumulative.basic.spend_limit = Some(limit.clone());
        let mut cumulative = Allowance::CumulativePeriodic(cumulative);

        let mut delayed = mock_delayed_allowance();
        delayed.basic.spend_limit = Some(limit.clone());
        let mut delayed = Allowance::Delayed(delayed);

        basic.increase_spend_limit(Uint128::new(100));
        classic.increase_spend_limit(Uint128::new(100));
        cumulative.increase_spend_limit(Uint128::new(100));
        delayed.increase_spend_limit(Uint128::new(100));

        assert_eq!(
            basic.basic().spend_limit.as_ref().unwrap().amount,
            limit.amount + Uint128::new(100)
        );
        assert_eq!(
            classic.basic().spend_limit.as_ref().unwrap().amount,
            limit.amount + Uint128::new(100)
        );
        assert_eq!(
            cumulative.basic().spend_limit.as_ref().unwrap().amount,
            limit.amount + Uint128::new(100)
        );
        assert_eq!(
            delayed.basic().spend_limit.as_ref().unwrap().amount,
            limit.amount + Uint128::new(100)
        );
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

            #[test]
            fn spend_limit_must_be_non_zero() {
                let mut allowance = mock_basic_allowance();

                let env = mock_env();

                // zero amount
                allowance.spend_limit = Some(coin(0, TEST_DENOM));
                assert!(allowance.validate(&env, TEST_DENOM).is_err());

                // non-zero amount
                allowance.spend_limit = Some(coin(69, TEST_DENOM));
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
            fn spend_limit_must_be_non_zero() {
                let mut allowance = mock_classic_periodic_allowance();

                // zero amount
                allowance.period_spend_limit = coin(0, TEST_DENOM);
                assert!(allowance.validate_new_inner(TEST_DENOM).is_err());

                // non-zero amount
                allowance.period_spend_limit = coin(69, TEST_DENOM);
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
            fn grant_must_be_non_zero() {
                let mut allowance = mock_cumulative_periodic_allowance();

                // zero amount
                allowance.period_grant = coin(0, TEST_DENOM);
                assert!(allowance.validate_new_inner(TEST_DENOM).is_err());

                // non-zero amount
                allowance.period_grant = coin(69, TEST_DENOM);
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
        use cosmwasm_std::testing::mock_env;

        #[test]
        fn basic_allowance() {
            let mut basic = Allowance::Basic(mock_basic_allowance());

            let og = basic.clone();

            // this is a no-op
            let env = mock_env();
            basic.set_initial_state(&env);
            assert_eq!(basic, og);
        }

        #[test]
        fn classic_periodic_allowance() {
            let mut inner = mock_classic_periodic_allowance();
            let mut cumulative = Allowance::ClassicPeriodic(inner.clone());

            let env = mock_env();

            let mut expected = inner.clone();

            // sets the spendable amount to min(basic_limit, period_limit)
            expected.period_can_spend = Some(expected.period_spend_limit.clone());

            // set period reset to current block time + period duration
            expected.period_reset_unix_timestamp =
                env.block.time.seconds() + expected.period_duration_secs;

            inner.set_initial_state(&env);
            assert_eq!(inner, expected);

            cumulative.set_initial_state(&env);
            assert_eq!(cumulative, Allowance::ClassicPeriodic(inner));
        }

        #[test]
        fn cumulative_periodic_allowance() {
            let mut inner = mock_cumulative_periodic_allowance();
            let mut cumulative = Allowance::CumulativePeriodic(inner.clone());

            let env = mock_env();

            // sets the last applied grant to current time and spendable to a single grant value
            let mut expected = inner.clone();
            expected.last_grant_applied_unix_timestamp = env.block.time.seconds();
            expected.spendable = Some(expected.period_grant.clone());

            inner.set_initial_state(&env);
            assert_eq!(inner, expected);

            cumulative.set_initial_state(&env);
            assert_eq!(cumulative, Allowance::CumulativePeriodic(inner));
        }

        #[test]
        fn delayed_allowance() {
            let mut delayed = Allowance::Delayed(mock_delayed_allowance());

            let og = delayed.clone();

            // this is a no-op
            let env = mock_env();
            delayed.set_initial_state(&env);
            assert_eq!(delayed, og);
        }
    }
}
