// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interface::{EmptyMessage, ServiceProviderRequest};

pub mod interface;

pub trait ServiceProvider<T: ServiceProviderRequest = EmptyMessage> {}
