// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::coconut::bandwidth::IssuanceBandwidthCredential;

pub struct State {
    pub voucher: IssuanceBandwidthCredential,
}

impl State {
    pub fn new(voucher: IssuanceBandwidthCredential) -> Self {
        State { voucher }
    }
}
