// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::derivable_impls)]
// MAX: surpressing warning for the moment, will be dealt with in a different PR (TODO)
use cosmwasm_schema::cw_serde;
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

    /// The epoch whose keys signers are issuing under.
    ///
    /// Only a concluded ceremony puts keys into service, so this is not `epoch_id - 1` while a
    /// ceremony runs: the id also increments when a ceremony *fails*, leaving an epoch behind
    /// with no keys and never any.
    ///
    /// `None` before the first ceremony concludes, and on an epoch stored before this field
    /// existed. Read that as "unknown", never as "none in service".
    #[serde(default)]
    pub keys_in_service: Option<EpochId>,

    /// The epoch [`Self::keys_in_service`] replaced, for the window in which a collection begun
    /// under it may still be completed. Likewise not `keys_in_service - 1`: any number of failed
    /// ceremonies may sit between the two.
    #[serde(default)]
    pub outgoing_keys: Option<EpochId>,
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
            // and one that has produced no keys, so it puts none into service. an epoch
            // succeeding another carries its predecessor's answer over - see `next_reset`
            keys_in_service: None,
            outgoing_keys: None,
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

            // and it is the moment this epoch's keys come into service, superseding whichever
            // generation held that position - not `epoch_id - 1`, which may be an epoch that a
            // failed ceremony abandoned
            self.outgoing_keys = self.keys_in_service;
            self.keys_in_service = Some(self.epoch_id);
        }

        self
    }

    pub fn next_reset(self, current_timestamp: Timestamp) -> Self {
        self.next_ceremony(false, current_timestamp)
    }

    pub fn next_resharing(self, current_timestamp: Timestamp) -> Self {
        self.next_ceremony(true, current_timestamp)
    }

    /// The epoch a fresh ceremony runs under, succeeding `self` whether it concluded or failed.
    fn next_ceremony(mut self, resharing: bool, current_timestamp: Timestamp) -> Self {
        self.epoch_id += 1;
        self.state_progress = Default::default();
        self.ceremony_concluded_at = None;

        // `keys_in_service` and `outgoing_keys` are deliberately left alone: a ceremony starting -
        // or failing - retires nothing, so the generation in service keeps serving until a
        // *successful* ceremony replaces it
        self.update(
            EpochState::PublicKeySubmission { resharing },
            current_timestamp,
        )
    }

    /// The epoch a fresh issuance is signed under, or `None` if no key generation is in service.
    pub fn issuing_epoch_id(&self) -> Option<EpochId> {
        // an epoch in service is its own answer, which keeps this honest against an epoch stored
        // before `keys_in_service` existed
        if self.state.is_final() {
            return Some(self.epoch_id);
        }

        self.keys_in_service
    }

    /// Whether the DKG ceremony that establishes `epoch_id`'s keys has finished, judged against
    /// `self` as the current epoch. Note this says nothing about whether that epoch is *over* -
    /// the current epoch's own ceremony is concluded for all of the time it is in use.
    ///
    /// Sitting below the current epoch is not enough: a failed ceremony moves the id on and
    /// leaves an epoch behind that concluded nothing, and calling that concluded would let
    /// callers cache its empty signer set for good. So the boundary is the epoch in service:
    /// nothing at or below it can ever change again, and no epoch that can still change reads
    /// concluded. (An epoch a failed ceremony abandoned below the boundary reads concluded
    /// too - its records are equally frozen, merely empty.) Callers working from a cached
    /// copy of the current epoch can only get a pessimistic answer out of a stale one, never
    /// a premature "yes".
    pub fn is_ceremony_concluded(&self, epoch_id: EpochId) -> bool {
        match self.issuing_epoch_id() {
            Some(in_service) => epoch_id <= in_service,
            // nothing has ever concluded, or a contract predating the field is mid-ceremony,
            // where refusing to cache is the safe answer
            None => false,
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

    /// An epoch whose ceremony concluded, reached the way the chain reaches it: by advancing into
    /// `InProgress`, never by being constructed there.
    fn concluded(epoch_id: EpochId) -> Epoch {
        epoch_at(
            epoch_id,
            EpochState::VerificationKeyFinalization { resharing: false },
        )
        .update(EpochState::InProgress, Timestamp::from_seconds(1234))
    }

    /// Nothing is in service until a ceremony finishes, and the epoch id cannot say so on its
    /// own: it counts ceremonies started, not key generations produced.
    #[test]
    fn no_keys_are_in_service_before_the_first_ceremony_concludes() {
        let uninitialised = epoch_at(0, EpochState::WaitingInitialisation);
        assert_eq!(None, uninitialised.keys_in_service);
        assert_eq!(None, uninitialised.issuing_epoch_id());

        let first_ceremony = epoch_at(0, EpochState::DealingExchange { resharing: false });
        assert_eq!(None, first_ceremony.issuing_epoch_id());
    }

    #[test]
    fn concluding_a_ceremony_puts_its_own_keys_into_service() {
        let concluded = concluded(3);

        assert_eq!(Some(3), concluded.keys_in_service);
        assert_eq!(Some(3), concluded.issuing_epoch_id());
    }

    /// B1: the epoch a ceremony is running for has no keys yet, so the generation already in
    /// service stays there for the duration.
    #[test]
    fn a_running_ceremony_leaves_the_keys_already_in_service_alone() {
        let mid_ceremony = concluded(3).next_reset(Timestamp::from_seconds(2000));

        assert_eq!(4, mid_ceremony.epoch_id);
        assert_eq!(Some(3), mid_ceremony.issuing_epoch_id());
    }

    #[test]
    fn resharing_also_leaves_the_keys_in_service_alone() {
        let mid_resharing = concluded(3).next_resharing(Timestamp::from_seconds(2000));

        assert_eq!(4, mid_resharing.epoch_id);
        assert_eq!(Some(3), mid_resharing.issuing_epoch_id());
    }

    /// A ceremony that fails takes the epoch id with it and leaves the keys where they were.
    /// Deriving the issuing epoch as `current - 1` instead names epoch 4 here: an epoch that
    /// never concluded, has no aggregate key, and never will.
    #[test]
    fn a_failed_ceremony_does_not_retire_the_keys_in_service() {
        // epoch 3's keys are in service; the ceremony for 4 runs, fails, and the contract resets
        // into 5 rather than remaining on an epoch it cannot complete
        let after_failure = concluded(3)
            .next_reset(Timestamp::from_seconds(2000))
            .next_reset(Timestamp::from_seconds(3000));

        assert_eq!(5, after_failure.epoch_id);
        assert_eq!(Some(3), after_failure.issuing_epoch_id());
    }

    /// However many failed on the way, the epoch superseded by a conclusion is the one that was
    /// actually in service - which the grace window then names, so a collection begun under it
    /// can still be completed.
    #[test]
    fn concluding_after_failures_supersedes_the_epoch_that_was_in_service() {
        let concluded_at_last = concluded(3)
            .next_reset(Timestamp::from_seconds(2000)) // ceremony 4, fails
            .next_reset(Timestamp::from_seconds(3000)) // ceremony 5, fails
            .next_reset(Timestamp::from_seconds(4000)) // ceremony 6, concludes
            .update(EpochState::InProgress, Timestamp::from_seconds(5000));

        assert_eq!(6, concluded_at_last.epoch_id);
        assert_eq!(Some(6), concluded_at_last.issuing_epoch_id());
        assert_eq!(Some(3), concluded_at_last.outgoing_keys);
    }

    /// A contract that has not been migrated yet reports nothing in service while a ceremony
    /// runs, which refuses issuance for its duration - the behaviour from before mid-ceremony
    /// issuance existed, and the safe direction. The conclusion then writes the real value.
    #[test]
    fn an_unseeded_epoch_mid_ceremony_reports_nothing_in_service() {
        let unseeded = epoch_at(5, EpochState::DealingExchange { resharing: false });

        assert_eq!(None, unseeded.keys_in_service);
        assert_eq!(None, unseeded.issuing_epoch_id());
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

    /// The epoch already stored on chain predates this field, so it has to keep deserialising -
    /// which is what lets the contract be migrated without touching stored data. Its conclusion
    /// time reads as unknown, deliberately: nothing on chain records it (the snapshot history is
    /// keyed by height, not time) and deriving it from the deadline would recover the last
    /// self-extension instead, which looks recent and would grant a window rather than withhold
    /// one. Callers must read `None` as "no window".
    #[test]
    fn an_epoch_stored_before_this_field_existed_still_loads() {
        let stored = r#"{
            "state": "in_progress",
            "epoch_id": 0,
            "state_progress": {
                "registered_dealers": 3,
                "registered_resharing_dealers": 0,
                "submitted_dealings": 15,
                "submitted_key_shares": 3,
                "verified_keys": 3
            },
            "time_configuration": {
                "public_key_submission_time_secs": 600,
                "dealing_exchange_time_secs": 300,
                "verification_key_submission_time_secs": 300,
                "verification_key_validation_time_secs": 60,
                "verification_key_finalization_time_secs": 60,
                "in_progress_time_secs": 1209600
            },
            "deadline": "1750000000000000000"
        }"#;

        // the same entry point cw_storage_plus reads stored values through
        let epoch: Epoch = cosmwasm_std::from_json(stored).unwrap();
        assert_eq!(epoch.epoch_id, 0);
        assert!(epoch.state.is_in_progress());
        assert_eq!(epoch.ceremony_concluded_at, None);

        // which keys are in service is unrecorded too, but unlike the conclusion time it is
        // recoverable: an epoch in service is its own answer, so issuance keeps working against
        // a contract that has not been migrated yet
        assert_eq!(None, epoch.keys_in_service);
        assert_eq!(None, epoch.outgoing_keys);
        assert_eq!(Some(0), epoch.issuing_epoch_id());
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

    /// Signer sets are only settled once a ceremony finishes, and callers cache them per epoch
    /// with no expiry. Answering "concluded" for an epoch still mid-ceremony - or for one a
    /// failed ceremony abandoned - would have them remember a set that is empty or partial.
    #[test]
    fn a_ceremony_is_concluded_only_once_its_keys_are_in_service() {
        let current = concluded(5);

        // earlier ceremonies are finished, and later ones cannot have started
        assert!(current.is_ceremony_concluded(4));
        assert!(!current.is_ceremony_concluded(6));

        // the current epoch's own ceremony is concluded for all the time it is in use
        assert!(current.is_ceremony_concluded(5));

        // whichever phase a ceremony is in, its own epoch has not concluded
        for state in [
            EpochState::PublicKeySubmission { resharing: false },
            EpochState::DealingExchange { resharing: false },
            EpochState::VerificationKeySubmission { resharing: true },
            EpochState::VerificationKeyValidation { resharing: false },
            EpochState::VerificationKeyFinalization { resharing: false },
        ] {
            let in_flight = current
                .next_reset(Timestamp::from_seconds(2000))
                .update(state, Timestamp::from_seconds(2000));

            assert!(
                !in_flight.is_ceremony_concluded(6),
                "{state} was treated as a concluded ceremony"
            );
            // and the generation still in service remains concluded throughout
            assert!(in_flight.is_ceremony_concluded(5));
        }
    }

    /// An epoch a failed ceremony left behind sits *below* the current id while having concluded
    /// nothing, so proximity to the current epoch cannot be what settles this.
    #[test]
    fn an_epoch_abandoned_by_a_failed_ceremony_never_counts_as_concluded() {
        let after_failure = concluded(5)
            .next_reset(Timestamp::from_seconds(2000))
            .next_reset(Timestamp::from_seconds(3000));

        assert_eq!(7, after_failure.epoch_id);
        assert!(!after_failure.is_ceremony_concluded(7));
        assert!(!after_failure.is_ceremony_concluded(6));
        assert!(after_failure.is_ceremony_concluded(5));
    }
}
