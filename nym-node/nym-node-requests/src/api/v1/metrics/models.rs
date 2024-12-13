// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use mixing::*;
pub use session::*;
pub use verloc::*;

pub mod packets {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone, Copy)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct PacketsStats {
        pub ingress_mixing: IngressMixingStats,
        pub egress_mixing: EgressMixingStats,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Copy)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct IngressMixingStats {
        // forward hop packets (i.e. to mixnode)
        pub forward_hop_packets_received: usize,

        // final hop packets (i.e. to gateway)
        pub final_hop_packets_received: usize,

        // packets that failed to get unwrapped
        pub malformed_packets_received: usize,

        // (forward) packets that had invalid, i.e. too large, delays
        pub excessive_delay_packets: usize,

        // forward hop packets (i.e. to mixnode)
        pub forward_hop_packets_dropped: usize,

        // final hop packets (i.e. to gateway)
        pub final_hop_packets_dropped: usize,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Copy)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct EgressMixingStats {
        pub forward_hop_packets_sent: usize,

        pub forward_hop_packets_dropped: usize,

        pub ack_packets_sent: usize,
    }
}

pub mod mixing {
    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct LegacyMixingStats {
        #[serde(with = "time::serde::rfc3339")]
        pub update_time: OffsetDateTime,

        #[serde(with = "time::serde::rfc3339")]
        pub previous_update_time: OffsetDateTime,

        pub received_since_startup: u64,

        // note: sent does not imply forwarded. We don't know if it was delivered successfully
        pub sent_since_startup: u64,

        // we know for sure we dropped those packets
        pub dropped_since_startup: u64,

        pub received_since_last_update: u64,

        // note: sent does not imply forwarded. We don't know if it was delivered successfully
        pub sent_since_last_update: u64,

        // we know for sure we dropped those packets
        pub dropped_since_last_update: u64,
    }
}

pub mod verloc {
    use nym_crypto::asymmetric::ed25519::{self, serde_helpers::bs58_ed25519_pubkey};
    use serde::{Deserialize, Serialize};
    use std::time::Duration;
    use time::OffsetDateTime;
    #[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct VerlocNodeResult {
        #[serde(with = "bs58_ed25519_pubkey")]
        #[cfg_attr(feature = "openapi", schema(value_type = String))]
        pub node_identity: ed25519::PublicKey,
    }

    #[derive(Serialize, Deserialize, Default, Debug, Clone)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct VerlocStats {
        pub previous: VerlocResult,
        pub current: VerlocResult,
    }

    #[derive(Serialize, Deserialize, Default, Debug, Clone)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    #[serde(rename_all = "camelCase")]
    pub enum VerlocResult {
        Data(VerlocResultData),
        MeasurementInProgress,
        #[default]
        Unavailable,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct VerlocResultData {
        pub nodes_tested: usize,

        #[serde(with = "time::serde::rfc3339")]
        pub run_started: OffsetDateTime,

        #[serde(with = "time::serde::rfc3339::option")]
        pub run_finished: Option<OffsetDateTime>,

        pub results: Vec<VerlocNodeResult>,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct VerlocMeasurement {
        /// Minimum RTT duration it took to receive an echo packet.
        #[serde(serialize_with = "humantime_serde::serialize")]
        pub minimum: Duration,

        /// Average RTT duration it took to receive the echo packets.
        #[serde(serialize_with = "humantime_serde::serialize")]
        pub mean: Duration,

        /// Maximum RTT duration it took to receive an echo packet.
        #[serde(serialize_with = "humantime_serde::serialize")]
        pub maximum: Duration,

        /// The standard deviation of the RTT duration it took to receive the echo packets.
        #[serde(serialize_with = "humantime_serde::serialize")]
        pub standard_deviation: Duration,
    }
}

pub mod session {
    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct Session {
        pub duration_ms: u64,
        pub typ: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
    pub struct SessionStats {
        #[serde(with = "time::serde::rfc3339")]
        #[cfg_attr(feature = "openapi", schema(value_type = String))]
        pub update_time: OffsetDateTime,

        pub unique_active_users: u32,

        #[serde(default = "Vec::new")] // field was added later
        pub unique_active_users_hashes: Vec<String>,

        pub sessions: Vec<Session>,

        pub sessions_started: u32,

        pub sessions_finished: u32,
    }
}
