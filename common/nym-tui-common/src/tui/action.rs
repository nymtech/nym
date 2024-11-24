// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

#[derive(Debug)]
pub enum Action<T: AppAction> {
    Quit,

    AppDefined(T),
}

pub trait AppAction: Debug {}
