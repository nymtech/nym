// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::networking::message::NewDealingMessage;
use crate::dkg::smart_contract::watcher;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) enum Event {
    NewDealing(NewDealingMessage),
    DkgContractChange(watcher::Event),
}

impl Event {
    pub(crate) fn new_contract_change_event(event: watcher::Event) -> Self {
        Event::DkgContractChange(event)
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::NewDealing(new_dealing_message) => {
                write!(f, "NewDealingEvent ({})", new_dealing_message)
            }
            Event::DkgContractChange(contract_watcher_event) => {
                write!(f, "DkgContractChangeEvent ({})", contract_watcher_event)
            }
        }
    }
}
