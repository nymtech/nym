// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::derivable_impls)]
// MAX: surpressing warning for the moment, will be dealt with in a different PR (TODO)
use cosmwasm_schema::cw_serde;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub use crate::dealer::{DealerDetails, DealerRegistrationDetails, PagedDealerResponse};
pub use cosmwasm_std::{Addr, Coin, Timestamp};
pub use cw4::Cw4Contract;
pub use nym_contracts_common::dealings::ContractSafeBytes;

pub type EncodedBTEPublicKeyWithProof = String;
pub type EncodedBTEPublicKeyWithProofRef<'a> = &'a str;
pub type NodeIndex = u64;
pub type EpochId = u64;
pub type DealingIndex = u32;
// we really don't need to hold more data than that (even u8 would have been enough),
// but explicitly make it different type than `DealingIndex` so type system would detect any
// accidental misuses
pub type ChunkIndex = u16;
pub type PartialContractDealingData = ContractSafeBytes;

#[cw_serde]
#[derive(Copy, Default)]
pub struct StateAdvanceResponse {
    pub current_state: EpochState,
    pub progress: StateProgress,
    pub deadline: Option<Timestamp>,
    pub reached_deadline: bool,
    pub is_complete: bool,
}

impl StateAdvanceResponse {
    pub fn can_advance(&self) -> bool {
        self.reached_deadline || self.is_complete
    }
}

#[cw_serde]
#[derive(Copy)]
pub struct TimeConfiguration {
    // The time sign-up is open for dealers to join
    pub public_key_submission_time_secs: u64,
    pub dealing_exchange_time_secs: u64,
    pub verification_key_submission_time_secs: u64,
    pub verification_key_validation_time_secs: u64,
    pub verification_key_finalization_time_secs: u64,
    /// Formerly the length of the `InProgress` phase, after which the epoch re-saved itself with
    /// a fresh deadline while rotating nothing. That self-extension is gone, so this is now
    /// unused; it stays because it is serialised inside every stored [`Epoch`].
    #[deprecated(note = "the InProgress phase no longer expires, so this value governs nothing")]
    pub in_progress_time_secs: u64,
}

impl TimeConfiguration {
    pub fn state_duration(&self, state: EpochState) -> Option<u64> {
        match state {
            EpochState::WaitingInitialisation => None,
            EpochState::PublicKeySubmission { .. } => Some(self.public_key_submission_time_secs),
            EpochState::DealingExchange { .. } => Some(self.dealing_exchange_time_secs),
            EpochState::VerificationKeySubmission { .. } => {
                Some(self.verification_key_submission_time_secs)
            }
            EpochState::VerificationKeyValidation { .. } => {
                Some(self.verification_key_validation_time_secs)
            }
            EpochState::VerificationKeyFinalization { .. } => {
                Some(self.verification_key_finalization_time_secs)
            }
            // the state machine stops here: rotation is an admin action, so there is no later
            // state for a deadline to lead to. One left behind would only go stale and then read
            // as "expired" to everything treating it as a cache bound.
            EpochState::InProgress => None,
        }
    }
}

impl FromStr for TimeConfiguration {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let times = s
            .split(',')
            .map(|t| t.parse())
            .collect::<Result<Vec<u64>, _>>()
            .map_err(|_| String::from("Could not parse string"))?;
        if times.len() != 6 {
            Err(String::from("Not enough time specified"))
        } else {
            // the vestigial field still has to be populated: it is part of the serialised form
            #[allow(deprecated)]
            Ok(TimeConfiguration {
                public_key_submission_time_secs: times[0],
                dealing_exchange_time_secs: times[1],
                verification_key_submission_time_secs: times[2],
                verification_key_validation_time_secs: times[3],
                verification_key_finalization_time_secs: times[4],
                in_progress_time_secs: times[5],
            })
        }
    }
}

impl Default for TimeConfiguration {
    // as above: written so the serialised form stays complete, never read
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            public_key_submission_time_secs: 60 * 10,      // 10 minutes
            dealing_exchange_time_secs: 60 * 5,            // 5 minutes
            verification_key_submission_time_secs: 60 * 5, // 5 minutes
            verification_key_validation_time_secs: 60,     // 1 minute
            verification_key_finalization_time_secs: 60,   // 1 minute
            in_progress_time_secs: 60 * 60 * 24 * 14,      // 2 weeks
        }
    }
}

#[cw_serde]
pub struct State {
    pub mix_denom: String,
    pub multisig_addr: Addr,
    pub group_addr: Cw4Contract,

    /// Specifies the number of elements in the derived keys
    pub key_size: u32,
}

#[cw_serde]
#[derive(Copy, Default)]
pub struct StateProgress {
    /// Counts the number of dealers that have registered in this epoch.
    // ideally we want to have here all group members
    pub registered_dealers: u32,

    /// Counts the number of resharing dealers that have registered in this epoch.
    /// This field is only populated during a resharing exchange.
    /// It is always <= registered_dealers.
    pub registered_resharing_dealers: u32,

    /// Counts the number of fully received dealings (i.e. full chunks) from all the allowed dealers.
    // we expect registered_dealers * state.key_size number of dealings here (each dealer has to submit key_size number of dealings)
    pub submitted_dealings: u32,

    /// Counts the number of submitted verification key shared from the dealers.
    // we expect registered_dealers number of keys here
    pub submitted_key_shares: u32,

    /// Counts the number of verified key shares.
    // we expect submitted_key_shares number of verified keys here
    pub verified_keys: u32,
}

#[cw_serde]
#[derive(Copy, Default)]
pub struct Epoch {
    pub state: EpochState,
    pub epoch_id: EpochId,
    pub state_progress: StateProgress,
    pub time_configuration: TimeConfiguration,

    #[serde(alias = "finish_timestamp")]
    pub deadline: Option<Timestamp>,

    /// When this epoch's ceremony finished, i.e. when its keys came into service.
    ///
    /// Nothing else on chain records this: the epoch id increments when a ceremony *starts*, and
    /// [`Self::deadline`] is per-state. Anything sized against the age of the current keys needs
    /// it - the window in which a signer still honours the epoch just superseded, and any pruning
    /// of retired key material.
    ///
    /// `None` for an epoch mid-ceremony, and for one that concluded before this field existed.
    #[serde(default)]
    pub ceremony_concluded_at: Option<Timestamp>,
}

impl Epoch {
    pub fn new(
        state: EpochState,
        epoch_id: u64,
        time_configuration: TimeConfiguration,
        current_timestamp: Timestamp,
    ) -> Self {
        let duration = time_configuration.state_duration(state);

        Epoch {
            state,
            epoch_id,
            state_progress: Default::default(),
            time_configuration,
            deadline: duration.map(|d| current_timestamp.plus_seconds(d)),
            // a freshly constructed epoch is one whose ceremony is about to run
            ceremony_concluded_at: None,
        }
    }

    pub fn update(mut self, next_state: EpochState, current_timestamp: Timestamp) -> Self {
        self.state = next_state;
        let duration = self.time_configuration.state_duration(next_state);
        self.deadline = duration.map(|d| current_timestamp.plus_seconds(d));

        // reaching `InProgress` *is* the ceremony concluding, and the state machine stops here,
        // so this is written exactly once per key generation
        if next_state.is_in_progress() {
            self.ceremony_concluded_at = Some(current_timestamp);
        }

        self
    }

    pub fn next_reset(self, current_timestamp: Timestamp) -> Self {
        Epoch::new(
            EpochState::PublicKeySubmission { resharing: false },
            self.epoch_id + 1,
            self.time_configuration,
            current_timestamp,
        )
    }

    pub fn next_resharing(self, current_timestamp: Timestamp) -> Self {
        Epoch::new(
            EpochState::PublicKeySubmission { resharing: true },
            self.epoch_id + 1,
            self.time_configuration,
            current_timestamp,
        )
    }

    /// Whether the DKG ceremony that establishes `epoch_id`'s keys has finished, judged against
    /// `self` as the current epoch. Note this says nothing about whether that epoch is *over* -
    /// the current epoch's own ceremony is concluded for all of the time it is in use.
    ///
    /// The ceremony of any epoch before the current one has necessarily finished, and one that
    /// has not been reached yet certainly has not started. Callers working from a cached copy of
    /// the current epoch can only get a pessimistic answer out of a stale one, never a premature
    /// "yes".
    pub fn is_ceremony_concluded(&self, epoch_id: EpochId) -> bool {
        match epoch_id.cmp(&self.epoch_id) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => self.state.is_in_progress(),
        }
    }

    pub fn final_timestamp_secs(&self) -> Option<u64> {
        let mut finish = self.deadline?.seconds();
        let time_configuration = self.time_configuration;
        let mut curr_epoch_state = self.state;
        while let Some(state) = curr_epoch_state.next() {
            curr_epoch_state = state;
            let adding = match curr_epoch_state {
                EpochState::WaitingInitialisation => return None,
                EpochState::PublicKeySubmission { .. } => {
                    time_configuration.public_key_submission_time_secs
                }
                EpochState::DealingExchange { .. } => time_configuration.dealing_exchange_time_secs,
                EpochState::VerificationKeySubmission { .. } => {
                    time_configuration.verification_key_submission_time_secs
                }
                EpochState::VerificationKeyValidation { .. } => {
                    time_configuration.verification_key_validation_time_secs
                }
                EpochState::VerificationKeyFinalization { .. } => {
                    time_configuration.verification_key_finalization_time_secs
                }
                EpochState::InProgress => 0,
            };
            finish += adding;
        }
        Some(finish)
    }
}

// currently (it is still extremely likely to change, we might be able to get rid of verification key-related complaints),
// the epoch can be in the following states (in order):
// 0. WaitingInitialisation -> the contract has been instantiated, but awaits for the admin to kick off the process (group members might still be getting added)
// 1. PublicKeySubmission -> potential dealers are submitting their BTE and ed25519 public keys to participate in dealing exchange
// 2. DealingExchange -> the actual (off-chain) dealing exchange is happening
// 3. ComplaintSubmission -> receivers submitting evidence of other dealers sending malformed data
// 4. ComplaintVoting -> (if any complaints were submitted) receivers voting on the validity of the evidence provided
// 5. VerificationKeySubmission -> receivers submitting their partial (and master) verification keys
// 6. VerificationKeyMismatchSubmission -> receivers / watchers raising issue that the submitted VK are mismatched with their local derivations
// 7. VerificationKeyMismatchVoting -> (if any complaints were submitted) receivers voting on received mismatches
// 8. InProgress -> all receivers have all their secrets derived and all is good
//
// Note: It's important that the variant ordering is not changed otherwise it would mess up the derived `PartialOrd`
#[cw_serde]
#[derive(Copy, Default)]
pub enum EpochState {
    #[default]
    WaitingInitialisation,
    PublicKeySubmission {
        resharing: bool,
    },
    DealingExchange {
        resharing: bool,
    },
    VerificationKeySubmission {
        resharing: bool,
    },
    VerificationKeyValidation {
        resharing: bool,
    },
    VerificationKeyFinalization {
        resharing: bool,
    },
    InProgress,
}

impl Display for EpochState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochState::WaitingInitialisation => write!(f, "Waiting for initialisation"),
            EpochState::PublicKeySubmission { resharing } => {
                write!(f, "PublicKeySubmission (resharing: {resharing})")
            }
            EpochState::DealingExchange { resharing } => {
                write!(f, "DealingExchange (resharing: {resharing})")
            }
            EpochState::VerificationKeySubmission { resharing } => {
                write!(f, "VerificationKeySubmission (resharing: {resharing})")
            }
            EpochState::VerificationKeyValidation { resharing } => {
                write!(f, "VerificationKeyValidation (resharing: {resharing})")
            }
            EpochState::VerificationKeyFinalization { resharing } => {
                write!(f, "VerificationKeyFinalization (resharing: {resharing})")
            }
            EpochState::InProgress => write!(f, "InProgress"),
        }
    }
}

impl EpochState {
    pub fn first() -> Self {
        EpochState::PublicKeySubmission { resharing: false }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            EpochState::WaitingInitialisation => None,
            EpochState::PublicKeySubmission { resharing } => {
                Some(EpochState::DealingExchange { resharing })
            }
            EpochState::DealingExchange { resharing } => {
                Some(EpochState::VerificationKeySubmission { resharing })
            }
            EpochState::VerificationKeySubmission { resharing } => {
                Some(EpochState::VerificationKeyValidation { resharing })
            }
            EpochState::VerificationKeyValidation { resharing } => {
                Some(EpochState::VerificationKeyFinalization { resharing })
            }
            EpochState::VerificationKeyFinalization { .. } => Some(EpochState::InProgress),
            EpochState::InProgress => None,
        }
    }

    pub fn all_until(&self, end: Self) -> Vec<Self> {
        let mut states = vec![*self];
        while states.last().unwrap() != &end {
            let next_state = states.last().unwrap().next().expect("somehow reached the end of state diff -> this should be impossible under any circumstances!");
            states.push(next_state);
        }

        states
    }

    pub fn is_final(&self) -> bool {
        *self == EpochState::InProgress
    }

    pub fn is_in_progress(&self) -> bool {
        matches!(self, EpochState::InProgress)
    }

    pub fn is_dealing_exchange(&self) -> bool {
        matches!(self, EpochState::DealingExchange { .. })
    }

    pub fn is_waiting_initialisation(&self) -> bool {
        matches!(self, EpochState::WaitingInitialisation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch_at(epoch_id: EpochId, state: EpochState) -> Epoch {
        Epoch::new(
            state,
            epoch_id,
            TimeConfiguration::default(),
            Timestamp::from_seconds(0),
        )
    }

    /// When a ceremony concluded is the only record of when a key generation came into service.
    /// Nothing else on chain carries it: the epoch id increments when a ceremony *starts*, and
    /// the deadline is per-state. Sizing anything against the age of the current keys - the
    /// issuance grace window, pruning of retired keys - depends on this being recorded.
    #[test]
    fn entering_in_progress_records_when_the_ceremony_concluded() {
        let mid_ceremony = epoch_at(
            5,
            EpochState::VerificationKeyFinalization { resharing: false },
        );
        assert_eq!(mid_ceremony.ceremony_concluded_at, None);

        let concluded = mid_ceremony.update(EpochState::InProgress, Timestamp::from_seconds(1234));
        assert_eq!(
            concluded.ceremony_concluded_at,
            Some(Timestamp::from_seconds(1234))
        );
    }

    /// An epoch in progress is where the state machine stops: rotation is an admin action, so
    /// there is no later state to hold a deadline for. Leaving one behind would also go stale and
    /// then read as "expired" to everything that treats it as a cache bound.
    #[test]
    fn an_epoch_in_progress_has_no_deadline() {
        let concluded = epoch_at(
            5,
            EpochState::VerificationKeyFinalization { resharing: false },
        )
        .update(EpochState::InProgress, Timestamp::from_seconds(1234));

        assert_eq!(concluded.deadline, None);
    }

    /// A fresh ceremony has not concluded, whatever the epoch it replaced had recorded.
    #[test]
    fn a_reset_clears_the_recorded_conclusion() {
        let concluded = epoch_at(
            5,
            EpochState::VerificationKeyFinalization { resharing: false },
        )
        .update(EpochState::InProgress, Timestamp::from_seconds(1234));

        let reset = concluded.next_reset(Timestamp::from_seconds(2000));
        assert_eq!(reset.ceremony_concluded_at, None);
    }

    /// Signer sets are only settled once a ceremony finishes, and callers cache them per epoch.
    /// Answering "concluded" for an epoch still mid-ceremony would have them remember a set
    /// that is empty or partial.
    #[test]
    fn a_ceremony_is_concluded_only_once_its_epoch_is_in_progress() {
        let current = epoch_at(5, EpochState::InProgress);

        // earlier ceremonies are finished by definition, and later ones cannot have started
        assert!(current.is_ceremony_concluded(4));
        assert!(!current.is_ceremony_concluded(6));

        // the current epoch's own ceremony is concluded for all the time it is in use
        assert!(current.is_ceremony_concluded(5));
        for state in [
            EpochState::WaitingInitialisation,
            EpochState::PublicKeySubmission { resharing: false },
            EpochState::DealingExchange { resharing: false },
            EpochState::VerificationKeySubmission { resharing: true },
            EpochState::VerificationKeyValidation { resharing: false },
            EpochState::VerificationKeyFinalization { resharing: false },
        ] {
            assert!(
                !epoch_at(5, state).is_ceremony_concluded(5),
                "{state} was treated as a concluded ceremony"
            );
        }
    }
}
