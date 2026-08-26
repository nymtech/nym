// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytes::{BufMut, BytesMut};
use std::fmt::{Debug, Formatter};

use nym_common::debug::format_debug_bytes;

pub use error::MalformedLpPacketError;
pub use frame::{ForwardPacketData, LpFrame};
pub use header::{InnerHeader, LpHeader, OuterHeader};

pub mod error;
pub mod frame;
pub mod header;

pub mod version {
    /// The initial version of the Lewes Protocol, and the only inner-header layout implemented.
    pub const V1: u8 = 1;

    /// The current version of the Lewes Protocol that is put into each new constructed header.
    pub const CURRENT: u8 = V1;

    /// Every version this build can still speak, and therefore everything a responder will accept
    /// during negotiation. Must contain [`CURRENT`].
    ///
    /// Deliberately spelled out rather than derived from [`CURRENT`]: bumping the current version
    /// must not silently drop support for the versions already deployed in the field.
    ///
    /// Adding a version here is necessary but not sufficient. It also needs its own PSQ session
    /// context and AAD constants with arms in both `build_psq_principal` functions, which
    /// otherwise reject it right after negotiation succeeds, and an [`super::InnerHeader::parse`]
    /// branch if it changes the header layout.
    pub const SUPPORTED: &[u8] = &[V1];

    pub mod node_compatibility {
        /// Indicates the initial version where LP has been introduced, alongside the KKT
        /// handshake it is built on.
        /// 1.27.0 Raclette release
        pub const INTRODUCTION: semver::Version = semver::Version::new(1, 27, 0);
    }

    /// Determine the LP protocol version spoken by a node of the given build version.
    ///
    /// Nodes do not advertise this directly, so it is derived the same way the KKT ciphersuite
    /// is. Returns `None` if the node predates LP entirely.
    pub fn from_node_version(semver: semver::Version) -> Option<u8> {
        if semver < node_compatibility::INTRODUCTION {
            // node can't possibly speak LP
            return None;
        }
        // currently there are no other branches known to the client
        // once a new version is introduced, follow the pattern implemented in
        // `common/authenticator-requests/src/version.rs`
        Some(V1)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_current_version_is_one_we_support() {
            // a `CURRENT` outside `SUPPORTED` means we'd propose a version we then refuse
            assert!(SUPPORTED.contains(&CURRENT));
        }

        #[test]
        fn nodes_predating_lp_speak_no_version() {
            for too_old in ["1.26.9", "1.0.0", "0.1.0"] {
                assert_eq!(None, from_node_version(too_old.parse().unwrap()));
            }
        }

        #[test]
        fn nodes_from_the_introduction_onwards_speak_v1() {
            for supported in ["1.27.0", "1.38.0", "2.0.0"] {
                assert_eq!(Some(V1), from_node_version(supported.parse().unwrap()));
            }
        }

        #[test]
        fn a_prerelease_of_the_introduction_is_treated_as_predating_lp() {
            // semver orders pre-releases below their release, so an rc of the introduction
            // resolves to `None`. This matches `Ciphersuite::from_node_version`, which uses the
            // same comparison and would already have rejected such a node before we get here.
            assert_eq!(None, from_node_version("1.27.0-rc.1".parse().unwrap()));
        }
    }
}

#[allow(dead_code)]
pub(crate) const UDP_HEADER_LEN: usize = 8;
#[allow(dead_code)]
pub(crate) const IP_HEADER_LEN: usize = 40; // v4 - 20, v6 - 40
#[allow(dead_code)]
pub(crate) const MTU: usize = 1500;
#[allow(dead_code)]
pub(crate) const UDP_OVERHEAD: usize = UDP_HEADER_LEN + IP_HEADER_LEN;
#[allow(dead_code)]
pub(crate) const UDP_PAYLOAD_SIZE: usize = MTU - UDP_OVERHEAD;

#[derive(Clone)]
pub struct EncryptedLpPacket {
    // The outer header that's sent in plaintext
    pub(crate) outer_header: OuterHeader,

    // The ciphertext containing the inner header and the payload
    pub(crate) ciphertext: Vec<u8>,
}

impl std::fmt::Debug for EncryptedLpPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

impl EncryptedLpPacket {
    pub fn new(outer_header: OuterHeader, ciphertext: Vec<u8>) -> EncryptedLpPacket {
        EncryptedLpPacket {
            outer_header,
            ciphertext,
        }
    }

    pub fn encoded_length(&self) -> usize {
        OuterHeader::SIZE + self.ciphertext.len()
    }

    pub(crate) fn debug_bytes(&self) -> Vec<u8> {
        let mut bytes = BytesMut::new();
        self.encode(&mut bytes);
        bytes.freeze().to_vec()
    }

    pub fn encode(&self, dst: &mut BytesMut) {
        self.outer_header.encode(dst);
        dst.put_slice(&self.ciphertext)
    }

    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    pub fn outer_header(&self) -> OuterHeader {
        self.outer_header
    }
}

#[derive(Clone, PartialEq)]
pub struct LpPacket {
    pub(crate) header: LpHeader,
    pub(crate) frame: LpFrame,
}

impl Debug for LpPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_debug_bytes(&self.debug_bytes())?)
    }
}

impl LpPacket {
    pub fn new(header: LpHeader, frame: LpFrame) -> Self {
        Self { header, frame }
    }

    pub fn frame(&self) -> &LpFrame {
        &self.frame
    }

    pub fn into_frame(self) -> LpFrame {
        self.frame
    }

    pub fn header(&self) -> &LpHeader {
        &self.header
    }

    pub(crate) fn debug_bytes(&self) -> Vec<u8> {
        let mut bytes = BytesMut::new();
        self.dbg_encode(&mut bytes);
        bytes.freeze().to_vec()
    }

    pub(crate) fn dbg_encode(&self, dst: &mut BytesMut) {
        self.header.dbg_encode(dst);
        self.frame.encode(dst)
    }
}
