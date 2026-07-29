// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Message)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeDescription {
    /// moniker defines a human-readable name for the node.
    #[prost(string, tag = "1")]
    pub moniker: String,

    /// website defines an optional website link.
    #[prost(string, tag = "2")]
    pub website: String,

    /// security contact defines an optional email for security contact.
    #[prost(string, tag = "3")]
    pub security_contact: String,

    /// details define other optional details.
    #[prost(string, tag = "4")]
    pub details: String,
}

// These tests exercise the payload codec (prost) against `NodeDescription`, whose
// field set is stable, rather than the empty `SphinxKey`/`Wireguard` placeholders -
// so round-trip, determinism, and forward-compatibility are actually observable.
#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> NodeDescription {
        NodeDescription {
            moniker: "node-1".to_string(),
            website: "https://nym.example".to_string(),
            security_contact: "security@nym.example".to_string(),
            details: "an example node".to_string(),
        }
    }

    #[test]
    fn round_trips_through_prost() {
        let original = sample();
        let decoded = NodeDescription::decode(original.encode_to_vec().as_slice())
            .expect("a payload must decode from the bytes it encoded to");

        assert_eq!(decoded.moniker, original.moniker);
        assert_eq!(decoded.website, original.website);
        assert_eq!(decoded.security_contact, original.security_contact);
        assert_eq!(decoded.details, original.details);
    }

    #[test]
    fn added_field_is_ignored_by_older_reader() {
        // A future payload version that appends a field under a fresh tag (5), keeping
        // the existing tags 1-4 unchanged - the forward-compatible way to grow a payload.
        #[derive(Clone, PartialEq, Message)]
        struct NodeDescriptionNext {
            #[prost(string, tag = "1")]
            moniker: String,
            #[prost(string, tag = "2")]
            website: String,
            #[prost(string, tag = "3")]
            security_contact: String,
            #[prost(string, tag = "4")]
            details: String,
            #[prost(string, tag = "5")]
            new_field: String,
        }

        let extended = NodeDescriptionNext {
            moniker: "node-1".to_string(),
            website: "https://nym.example".to_string(),
            security_contact: "security@nym.example".to_string(),
            details: "an example node".to_string(),
            new_field: "unknown-to-the-old-reader".to_string(),
        };

        // The older reader, which predates tag 5, still decodes and simply drops the
        // unknown field.
        let decoded = NodeDescription::decode(extended.encode_to_vec().as_slice())
            .expect("an older reader must tolerate an unknown appended field");

        assert_eq!(decoded.moniker, extended.moniker);
        assert_eq!(decoded.website, extended.website);
        assert_eq!(decoded.security_contact, extended.security_contact);
        assert_eq!(decoded.details, extended.details);
    }
}
