// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{Addr, BlockHeight, DealerDetails, Epoch};
use contracts_common::commitment::ContractSafeCommitment;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) struct Event {
    pub(crate) height: BlockHeight,
    pub(crate) event_type: EventType,
}

impl Event {
    pub(crate) fn new(height: BlockHeight, event_type: EventType) -> Self {
        Event { height, event_type }
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SmartContractWatcherEvent at height {}. {}",
            self.height, self.event_type
        )
    }
}

#[derive(Debug)]
pub(crate) enum CommitmentChange {
    Addition {
        address: Addr,
        commitment: ContractSafeCommitment,
    },
    Removal {
        address: Addr,
    },
    Update {
        address: Addr,
        commitment: ContractSafeCommitment,
    },
}

#[derive(Debug)]
pub(crate) enum DealerChange {
    Addition { details: DealerDetails },
    Removal { address: Addr },
}

#[derive(Debug)]
pub(crate) enum EventType {
    NoChange,
    NewKeySubmission,
    DealerSetChange { changes: Vec<DealerChange> },
    KnownCommitmentsChange { changes: Vec<CommitmentChange> },
    NewDealingCommitment { epoch: Epoch },
}

impl Display for EventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventType - ")?;
        match self {
            EventType::NewKeySubmission => write!(f, "NewKeySubmission"),
            EventType::DealerSetChange { changes } => {
                write!(f, "DealerSetChange with {} changes", changes.len())
            }
            EventType::KnownCommitmentsChange { changes } => {
                write!(f, "KnownCommitmentsChange with {} changes", changes.len())
            }
            EventType::NewDealingCommitment { epoch } => {
                write!(f, "NewDealingCommitment for epoch {}", epoch.id)
            }
            EventType::NoChange => write!(f, "NoChange"),
        }
    }
}
