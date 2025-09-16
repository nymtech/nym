// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{net::IpAddr, path::PathBuf};

use nym_authenticator_requests::AuthenticatorVersion;
use nym_sdk::mixnet::{NodeIdentity, Recipient};

// IMO this should live somewhere else but alas
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NymNode {
    pub identity: NodeIdentity,
    pub ip_address: IpAddr,
    pub ipr_address: Option<Recipient>,
    pub authenticator_address: Option<Recipient>,
    pub version: AuthenticatorVersion,
}
pub struct RegistrationClientConfig {
    pub(crate) entry: NymNode,
    pub(crate) exit: NymNode,
    pub(crate) two_hops: bool,
    pub(crate) data_path: Option<PathBuf>,
}
