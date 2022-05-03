// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::networking::message::{NewDealingMessage, RemoteDealingRequestMessage};

#[derive(Debug)]
pub(crate) enum Event {
    NewDealing(NewDealingMessage),
    NewDealingRequest(RemoteDealingRequestMessage),
}
