// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_interface::Parameters;
use nym_credentials::coconut::bandwidth::BandwidthVoucher;

pub struct State {
    pub voucher: BandwidthVoucher,
    pub params: Parameters,
}

impl State {
    pub fn new(voucher: BandwidthVoucher) -> Self {
        State {
            voucher,
            params: BandwidthVoucher::default_parameters(),
        }
    }
}
