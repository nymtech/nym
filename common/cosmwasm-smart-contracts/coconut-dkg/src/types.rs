// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub use crate::dealer::{DealerDetails, DealerRegistrationDetails, PagedDealerResponse};
pub use contracts_common::dealings::ContractSafeBytes;
pub use cosmwasm_std::{Addr, Coin, Timestamp};
pub use cw4::Cw4Contract;

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
#[derive(Copy)]
pub struct TimeConfiguration {
    // The time sign-up is open for dealers to join
    pub public_key_submission_time_secs: u64,
    pub dealing_exchange_time_secs: u64,
    pub verification_key_submission_time_secs: u64,
    pub verification_key_validation_time_secs: u64,
    pub verification_key_finalization_time_secs: u64,
    // The time an epoch lasts
    pub in_progress_time_secs: u64,
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
    // ideally we want to have here all group members
    pub registered_dealers: u32,

    // we expect registered_dealers * state.key_size number of dealings here (each dealer has to submit key_size number of dealings)
    pub submitted_dealings: u32,

    // we expect registered_dealers number of keys here
    pub submitted_key_shares: u32,

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
}

impl Epoch {
    pub fn new(
        state: EpochState,
        epoch_id: u64,
        time_configuration: TimeConfiguration,
        current_timestamp: Timestamp,
    ) -> Self {
        let duration = match state {
            EpochState::WaitingInitialisation => None,
            EpochState::PublicKeySubmission { .. } => {
                Some(time_configuration.public_key_submission_time_secs)
            }
            EpochState::DealingExchange { .. } => {
                Some(time_configuration.dealing_exchange_time_secs)
            }
            EpochState::VerificationKeySubmission { .. } => {
                Some(time_configuration.verification_key_submission_time_secs)
            }
            EpochState::VerificationKeyValidation { .. } => {
                Some(time_configuration.verification_key_validation_time_secs)
            }
            EpochState::VerificationKeyFinalization { .. } => {
                Some(time_configuration.verification_key_finalization_time_secs)
            }
            EpochState::InProgress => Some(time_configuration.in_progress_time_secs),
        };
        Epoch {
            state,
            epoch_id,
            state_progress: Default::default(),
            time_configuration,
            deadline: duration.map(|d| current_timestamp.plus_seconds(d)),
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
                EpochState::InProgress { .. } => 0,
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
#[derive(Copy)]
pub enum EpochState {
    WaitingInitialisation,
    PublicKeySubmission { resharing: bool },
    DealingExchange { resharing: bool },
    VerificationKeySubmission { resharing: bool },
    VerificationKeyValidation { resharing: bool },
    VerificationKeyFinalization { resharing: bool },
    InProgress,
}

impl Default for EpochState {
    fn default() -> Self {
        Self::WaitingInitialisation
    }
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
}
