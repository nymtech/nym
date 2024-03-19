// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::old::v3::{
    AcknowledgementsV3, CoverTrafficV3, DebugConfigV3, GatewayConnectionV3,
    GatewayEndpointConfigV3, ReplySurbsV3, TopologyV3, TrafficV3,
};
use nym_sphinx_params::{PacketSize, PacketType};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

// 'DEBUG'
pub(crate) const DEFAULT_ACK_WAIT_MULTIPLIER: f64 = 1.5;

pub(crate) const DEFAULT_ACK_WAIT_ADDITION: Duration = Duration::from_millis(1_500);
pub(crate) const DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(200);
pub(crate) const DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(20);
pub(crate) const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(50);
pub(crate) const DEFAULT_TOPOLOGY_REFRESH_RATE: Duration = Duration::from_secs(5 * 60); // every 5min
pub(crate) const DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT: Duration = Duration::from_millis(5_000);
// Set this to a high value for now, so that we don't risk sporadic timeouts that might cause
// bought bandwidth tokens to not have time to be spent; Once we remove the gateway from the
// bandwidth bridging protocol, we can come back to a smaller timeout value
pub(crate) const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub(crate) const DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO: f64 = 0.70;

// reply-surbs related:

// define when to request
// clients/client-core/src/client/replies/reply_storage/surb_storage.rs
pub(crate) const DEFAULT_MINIMUM_REPLY_SURB_STORAGE_THRESHOLD: usize = 10;
pub(crate) const DEFAULT_MAXIMUM_REPLY_SURB_STORAGE_THRESHOLD: usize = 200;

// define how much to request at once
// clients/client-core/src/client/replies/reply_controller.rs
pub(crate) const DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE: u32 = 10;
pub(crate) const DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE: u32 = 100;

pub(crate) const DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE: u32 = 500;

pub(crate) const DEFAULT_MAXIMUM_REPLY_SURB_REREQUEST_WAITING_PERIOD: Duration =
    Duration::from_secs(10);
pub(crate) const DEFAULT_MAXIMUM_REPLY_SURB_DROP_WAITING_PERIOD: Duration =
    Duration::from_secs(5 * 60);

// 12 hours
pub(crate) const DEFAULT_MAXIMUM_REPLY_SURB_AGE: Duration = Duration::from_secs(12 * 60 * 60);

// 24 hours
pub(crate) const DEFAULT_MAXIMUM_REPLY_KEY_AGE: Duration = Duration::from_secs(24 * 60 * 60);

// aliases for backwards compatibility
pub type ConfigV1_1_20<T> = ConfigV2<T>;
pub type ClientV1_1_20<T> = ClientV2<T>;
pub type LoggingV1_1_20 = LoggingV2;
pub type DebugConfigV1_1_20 = DebugConfigV2;
pub type GatewayEndpointConfigV1_1_20 = GatewayEndpointConfigV2;

pub type TrafficV1_1_20 = TrafficV2;
pub type CoverTrafficV1_1_20 = CoverTrafficV2;
pub type GatewayConnectionV1_1_20 = GatewayConnectionV2;
pub type AcknowledgementsV1_1_20 = AcknowledgementsV2;
pub type TopologyV1_1_20 = TopologyV2;
pub type ReplySurbsV1_1_20 = ReplySurbsV2;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV2<T> {
    pub client: ClientV2<T>,

    #[serde(default)]
    pub logging: LoggingV2,
    #[serde(default)]
    pub debug: DebugConfigV2,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct GatewayEndpointConfigV2 {
    pub gateway_id: String,
    pub gateway_owner: String,
    pub gateway_listener: String,
}

impl From<GatewayEndpointConfigV2> for GatewayEndpointConfigV3 {
    fn from(value: GatewayEndpointConfigV2) -> Self {
        GatewayEndpointConfigV3 {
            gateway_id: value.gateway_id,
            gateway_owner: value.gateway_owner,
            gateway_listener: value.gateway_listener,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct ClientV2<T> {
    pub version: String,
    pub id: String,
    #[serde(default)]
    pub disabled_credentials_mode: bool,
    #[serde(alias = "validator_urls")]
    pub nyxd_urls: Vec<Url>,
    #[serde(alias = "validator_api_urls")]
    pub nym_api_urls: Vec<Url>,
    pub private_identity_key_file: PathBuf,
    pub public_identity_key_file: PathBuf,
    pub private_encryption_key_file: PathBuf,
    pub public_encryption_key_file: PathBuf,
    pub gateway_shared_key_file: PathBuf,
    pub ack_key_file: PathBuf,
    pub gateway_endpoint: GatewayEndpointConfigV2,
    pub database_path: PathBuf,
    #[serde(default)]
    pub reply_surb_database_path: PathBuf,
    pub nym_root_directory: PathBuf,

    #[serde(skip)]
    pub super_struct: PhantomData<T>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingV2 {}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TrafficV2 {
    #[serde(with = "humantime_serde")]
    pub average_packet_delay: Duration,
    #[serde(with = "humantime_serde")]
    pub message_sending_average_delay: Duration,
    pub disable_main_poisson_packet_distribution: bool,
    pub primary_packet_size: PacketSize,
    pub secondary_packet_size: Option<PacketSize>,
}

impl From<TrafficV2> for TrafficV3 {
    fn from(value: TrafficV2) -> Self {
        TrafficV3 {
            average_packet_delay: value.average_packet_delay,
            message_sending_average_delay: value.message_sending_average_delay,
            disable_main_poisson_packet_distribution: value
                .disable_main_poisson_packet_distribution,
            primary_packet_size: value.primary_packet_size,
            secondary_packet_size: value.secondary_packet_size,
            packet_type: PacketType::Mix,
        }
    }
}

impl Default for TrafficV2 {
    fn default() -> Self {
        TrafficV2 {
            average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            message_sending_average_delay: DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
            disable_main_poisson_packet_distribution: false,
            primary_packet_size: PacketSize::RegularPacket,
            secondary_packet_size: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CoverTrafficV2 {
    #[serde(with = "humantime_serde")]
    pub loop_cover_traffic_average_delay: Duration,
    pub cover_traffic_primary_size_ratio: f64,
    pub disable_loop_cover_traffic_stream: bool,
}

impl From<CoverTrafficV2> for CoverTrafficV3 {
    fn from(value: CoverTrafficV2) -> Self {
        CoverTrafficV3 {
            loop_cover_traffic_average_delay: value.loop_cover_traffic_average_delay,
            cover_traffic_primary_size_ratio: value.cover_traffic_primary_size_ratio,
            disable_loop_cover_traffic_stream: value.disable_loop_cover_traffic_stream,
        }
    }
}

impl Default for CoverTrafficV2 {
    fn default() -> Self {
        CoverTrafficV2 {
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            cover_traffic_primary_size_ratio: DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO,
            disable_loop_cover_traffic_stream: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GatewayConnectionV2 {
    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,
}

impl From<GatewayConnectionV2> for GatewayConnectionV3 {
    fn from(value: GatewayConnectionV2) -> Self {
        GatewayConnectionV3 {
            gateway_response_timeout: value.gateway_response_timeout,
        }
    }
}

impl Default for GatewayConnectionV2 {
    fn default() -> Self {
        GatewayConnectionV2 {
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AcknowledgementsV2 {
    #[serde(with = "humantime_serde")]
    pub average_ack_delay: Duration,
    pub ack_wait_multiplier: f64,
    #[serde(with = "humantime_serde")]
    pub ack_wait_addition: Duration,
}

impl From<AcknowledgementsV2> for AcknowledgementsV3 {
    fn from(value: AcknowledgementsV2) -> Self {
        AcknowledgementsV3 {
            average_ack_delay: value.average_ack_delay,
            ack_wait_multiplier: value.ack_wait_multiplier,
            ack_wait_addition: value.ack_wait_addition,
        }
    }
}

impl Default for AcknowledgementsV2 {
    fn default() -> Self {
        AcknowledgementsV2 {
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopologyV2 {
    #[serde(with = "humantime_serde")]
    pub topology_refresh_rate: Duration,
    #[serde(with = "humantime_serde")]
    pub topology_resolution_timeout: Duration,
    pub disable_refreshing: bool,
}

impl From<TopologyV2> for TopologyV3 {
    fn from(value: TopologyV2) -> Self {
        TopologyV3 {
            topology_refresh_rate: value.topology_refresh_rate,
            topology_resolution_timeout: value.topology_resolution_timeout,
            disable_refreshing: value.disable_refreshing,
        }
    }
}

impl Default for TopologyV2 {
    fn default() -> Self {
        TopologyV2 {
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            disable_refreshing: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReplySurbsV2 {
    pub minimum_reply_surb_storage_threshold: usize,
    pub maximum_reply_surb_storage_threshold: usize,
    pub minimum_reply_surb_request_size: u32,
    pub maximum_reply_surb_request_size: u32,
    pub maximum_allowed_reply_surb_request_size: u32,
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_rerequest_waiting_period: Duration,
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_drop_waiting_period: Duration,
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_age: Duration,
    #[serde(with = "humantime_serde")]
    pub maximum_reply_key_age: Duration,
}

impl From<ReplySurbsV2> for ReplySurbsV3 {
    fn from(value: ReplySurbsV2) -> Self {
        ReplySurbsV3 {
            minimum_reply_surb_storage_threshold: value.minimum_reply_surb_storage_threshold,
            maximum_reply_surb_storage_threshold: value.maximum_reply_surb_storage_threshold,
            minimum_reply_surb_request_size: value.minimum_reply_surb_request_size,
            maximum_reply_surb_request_size: value.maximum_reply_surb_request_size,
            maximum_allowed_reply_surb_request_size: value.maximum_allowed_reply_surb_request_size,
            maximum_reply_surb_rerequest_waiting_period: value
                .maximum_reply_surb_rerequest_waiting_period,
            maximum_reply_surb_drop_waiting_period: value.maximum_reply_surb_drop_waiting_period,
            maximum_reply_surb_age: value.maximum_reply_surb_age,
            maximum_reply_key_age: value.maximum_reply_key_age,
        }
    }
}

impl Default for ReplySurbsV2 {
    fn default() -> Self {
        ReplySurbsV2 {
            minimum_reply_surb_storage_threshold: DEFAULT_MINIMUM_REPLY_SURB_STORAGE_THRESHOLD,
            maximum_reply_surb_storage_threshold: DEFAULT_MAXIMUM_REPLY_SURB_STORAGE_THRESHOLD,
            minimum_reply_surb_request_size: DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE,
            maximum_reply_surb_request_size: DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE,
            maximum_allowed_reply_surb_request_size: DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE,
            maximum_reply_surb_rerequest_waiting_period:
                DEFAULT_MAXIMUM_REPLY_SURB_REREQUEST_WAITING_PERIOD,
            maximum_reply_surb_drop_waiting_period: DEFAULT_MAXIMUM_REPLY_SURB_DROP_WAITING_PERIOD,
            maximum_reply_surb_age: DEFAULT_MAXIMUM_REPLY_SURB_AGE,
            maximum_reply_key_age: DEFAULT_MAXIMUM_REPLY_KEY_AGE,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugConfigV2 {
    pub traffic: TrafficV2,
    pub cover_traffic: CoverTrafficV2,
    pub gateway_connection: GatewayConnectionV2,
    pub acknowledgements: AcknowledgementsV2,
    pub topology: TopologyV2,
    pub reply_surbs: ReplySurbsV2,
}

impl From<DebugConfigV2> for DebugConfigV3 {
    fn from(value: DebugConfigV2) -> Self {
        DebugConfigV3 {
            traffic: value.traffic.into(),
            cover_traffic: value.cover_traffic.into(),
            gateway_connection: value.gateway_connection.into(),
            acknowledgements: value.acknowledgements.into(),
            topology: value.topology.into(),
            reply_surbs: value.reply_surbs.into(),
        }
    }
}
