// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::coconut::bandwidth::IssuanceTicketBook;

pub struct State {
    pub voucher: IssuanceTicketBook,
}

impl State {
    pub fn new(voucher: IssuanceTicketBook) -> Self {
        State { voucher }
    }
}
