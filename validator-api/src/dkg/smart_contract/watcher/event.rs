// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{BlockHeight, DealerDetails};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) struct Event {
    height: BlockHeight,
    event_type: EventType,
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
pub(crate) enum EventType {
    NewDealerIdentity { details: DealerDetails },
    NewDealingCommitment,
}

impl Display for EventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventType - ")?;
        match self {
            EventType::NewDealerIdentity { details } => {
                write!(f, "NewDealerIdentity for {}", details.address)
            }
            EventType::NewDealingCommitment => write!(f, "NewDealingCommitment"),
        }
    }
}
