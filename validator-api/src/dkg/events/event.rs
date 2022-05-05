// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::networking::message::{NewDealingMessage, RemoteDealingRequestMessage};
use crate::dkg::smart_contract::watcher;

#[derive(Debug)]
pub(crate) enum Event {
    NewDealing(NewDealingMessage),
    NewDealingRequest(RemoteDealingRequestMessage),
    DkgContractChange(watcher::Event),
}

impl Event {
    pub(crate) fn name(&self) -> String {
        match self {
            Event::NewDealing(..) => "NewDealing".to_string(),
            Event::NewDealingRequest(..) => "NewDealingRequest".to_string(),

            Event::DkgContractChange(..) => "DkgContractChange".to_string(),
        }
    }
}
