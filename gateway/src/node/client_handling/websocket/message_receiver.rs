// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;

pub(crate) type MixMessageSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub(crate) type MixMessageReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;
