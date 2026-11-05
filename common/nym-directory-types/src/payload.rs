// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    LewesProtocolDetails, MixnetServiceProviders, NodeDescription, NodeInformation, SphinxKeys,
    Wireguard,
};
use nym_directory_contract_common::KnownLabel;
use prost::Message;

pub use prost::DecodeError;

/// A payload a node publishes under one of the directory's known labels, with its canonical
/// encoding: the bytes a node writes are exactly the bytes a reader decodes.
pub trait DirectoryEntryPayload: Message + Default {
    const LABEL: KnownLabel;

    fn to_canonical_bytes(&self) -> Vec<u8> {
        self.encode_to_vec()
    }

    fn from_canonical_bytes(data: &[u8]) -> Result<Self, DecodeError> {
        Self::decode(data)
    }
}

impl DirectoryEntryPayload for SphinxKeys {
    const LABEL: KnownLabel = KnownLabel::SphinxKeys;
}

impl DirectoryEntryPayload for NodeDescription {
    const LABEL: KnownLabel = KnownLabel::NodeDescription;
}

impl DirectoryEntryPayload for MixnetServiceProviders {
    const LABEL: KnownLabel = KnownLabel::MixnetServiceProviders;
}

impl DirectoryEntryPayload for Wireguard {
    const LABEL: KnownLabel = KnownLabel::Wireguard;
}

impl DirectoryEntryPayload for NodeInformation {
    const LABEL: KnownLabel = KnownLabel::NodeInformation;
}

impl DirectoryEntryPayload for LewesProtocolDetails {
    const LABEL: KnownLabel = KnownLabel::LewesProtocolDetails;
}
