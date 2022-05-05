// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Client;

pub(crate) struct Publisher<C> {
    client: Client<C>,
}
