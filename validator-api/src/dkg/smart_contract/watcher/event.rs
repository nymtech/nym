// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) enum Event {
    NewDealingCommitment,
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SmartContractWatcherEvent - ")?;
        match self {
            Event::NewDealingCommitment => write!(f, "NewDealingCommitment"),
        }
    }
}
