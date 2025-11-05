// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_provider_requests_common::{Protocol, ServiceProviderType};

pub mod conversion;
pub mod registration;
pub mod request;
pub mod response;
pub mod topup;
pub mod upgrade_mode_check;

pub const VERSION: u8 = 6;

pub const PROTOCOL: Protocol = Protocol::new(VERSION, ServiceProviderType::Authenticator);
