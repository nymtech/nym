// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;

pub(crate) type Dealer<'a> = &'a Addr;

mod constants;
pub mod contract;
mod dealers;
mod dealings;
mod epoch_state;
pub mod error;
mod state;
mod support;
mod verification_key_shares;
