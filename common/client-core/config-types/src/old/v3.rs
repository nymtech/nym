// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::old::v4::{
    AcknowledgementsV4, ClientV4, ConfigV4, CoverTrafficV4, DebugConfigV4, GatewayConnectionV4,
    ReplySurbsV4, TopologyV4, TrafficV4,
};
use crate::old::v5::GatewayEndpointConfigV5;
use nym_sphinx_params::{PacketSize, PacketType};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

// 'DEBUG'
const DEFAULT_ACK_WAIT_MULTIPLIER: f64 = 1.5;

const DEFAULT_ACK_WAIT_ADDITION: Duration = Duration::from_millis(1_500);
const DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(20);
const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(50);
const DEFAULT_TOPOLOGY_REFRESH_RATE: Duration = Duration::from_secs(5 * 60); // every 5min
const DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT: Duration = Duration::from_millis(5_000);
// Set this to a high value for now, so that we don't risk sporadic timeouts that might cause
// bought bandwidth tokens to not have time to be spent; Once we remove the gateway from the
// bandwidth bridging protocol, we can come back to a smaller timeout value
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

const DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO: f64 = 0.70;

// reply-surbs related:

// define when to request
// clients/client-core/src/client/replies/reply_storage/surb_storage.rs
const DEFAULT_MINIMUM_REPLY_SURB_STORAGE_THRESHOLD: usize = 10;
const DEFAULT_MAXIMUM_REPLY_SURB_STORAGE_THRESHOLD: usize = 200;

// define how much to request at once
// clients/client-core/src/client/replies/reply_controller.rs
const DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE: u32 = 10;
const DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE: u32 = 100;

const DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE: u32 = 500;

const DEFAULT_MAXIMUM_REPLY_SURB_REREQUEST_WAITING_PERIOD: Duration = Duration::from_secs(10);
const DEFAULT_MAXIMUM_REPLY_SURB_DROP_WAITING_PERIOD: Duration = Duration::from_secs(5 * 60);

// 12 hours
const DEFAULT_MAXIMUM_REPLY_SURB_AGE: Duration = Duration::from_secs(12 * 60 * 60);

// 24 hours
const DEFAULT_MAXIMUM_REPLY_KEY_AGE: Duration = Duration::from_secs(24 * 60 * 60);

// aliases for backwards compatibility
pub type ConfigV1_1_20_2 = ConfigV3;
pub type ClientV1_1_20_2 = ClientV3;
pub type DebugConfigV1_1_20_2 = DebugConfigV3;
pub type GatewayEndpointConfigV1_1_20_2 = GatewayEndpointConfigV3;

pub type TrafficV1_1_20_2 = TrafficV3;
pub type CoverTrafficV1_1_20_2 = CoverTrafficV3;
pub type GatewayConnectionV1_1_20_2 = GatewayConnectionV3;
pub type AcknowledgementsV1_1_20_2 = AcknowledgementsV3;
pub type TopologyV1_1_20_2 = TopologyV3;
pub type ReplySurbsV1_1_20_2 = ReplySurbsV3;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV3 {
    pub client: ClientV3,

    #[serde(default)]
    pub debug: DebugConfigV3,
}

impl From<ConfigV3> for ConfigV4 {
    fn from(value: ConfigV3) -> Self {
        ConfigV4 {
            client: value.client.into(),
            debug: value.debug.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct GatewayEndpointConfigV3 {
    /// gateway_id specifies ID of the gateway to which the client should send messages.
    /// If initially omitted, a random gateway will be chosen from the available topology.
    pub gateway_id: String,

    /// Address of the gateway owner to which the client should send messages.
    pub gateway_owner: String,

    /// Address of the gateway listener to which all client requests should be sent.
    pub gateway_listener: String,
}

impl From<GatewayEndpointConfigV3> for GatewayEndpointConfigV5 {
    fn from(value: GatewayEndpointConfigV3) -> Self {
        GatewayEndpointConfigV5 {
            gateway_id: value.gateway_id,
            gateway_owner: value.gateway_owner,
            gateway_listener: value.gateway_listener,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct ClientV3 {
    pub version: String,

    pub id: String,

    #[serde(default)]
    pub disabled_credentials_mode: bool,

    #[serde(alias = "validator_urls")]
    pub nyxd_urls: Vec<Url>,

    #[serde(alias = "validator_api_urls")]
    pub nym_api_urls: Vec<Url>,
    pub gateway_endpoint: GatewayEndpointConfigV3,
}

impl From<ClientV3> for ClientV4 {
    fn from(value: ClientV3) -> Self {
        ClientV4 {
            version: value.version,
            id: value.id,
            disabled_credentials_mode: value.disabled_credentials_mode,
            nyxd_urls: value.nyxd_urls,
            nym_api_urls: value.nym_api_urls,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TrafficV3 {
    #[serde(with = "humantime_serde")]
    pub average_packet_delay: Duration,
    #[serde(with = "humantime_serde")]
    pub message_sending_average_delay: Duration,
    pub disable_main_poisson_packet_distribution: bool,
    pub primary_packet_size: PacketSize,
    pub secondary_packet_size: Option<PacketSize>,
    pub packet_type: PacketType,
}

impl From<TrafficV3> for TrafficV4 {
    fn from(value: TrafficV3) -> Self {
        TrafficV4 {
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

impl Default for TrafficV3 {
    fn default() -> Self {
        TrafficV3 {
            average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            message_sending_average_delay: DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
            disable_main_poisson_packet_distribution: false,
            primary_packet_size: PacketSize::RegularPacket,
            secondary_packet_size: None,
            packet_type: PacketType::Mix,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CoverTrafficV3 {
    #[serde(with = "humantime_serde")]
    pub loop_cover_traffic_average_delay: Duration,
    pub cover_traffic_primary_size_ratio: f64,
    pub disable_loop_cover_traffic_stream: bool,
}

impl From<CoverTrafficV3> for CoverTrafficV4 {
    fn from(value: CoverTrafficV3) -> Self {
        CoverTrafficV4 {
            loop_cover_traffic_average_delay: value.loop_cover_traffic_average_delay,
            cover_traffic_primary_size_ratio: value.cover_traffic_primary_size_ratio,
            disable_loop_cover_traffic_stream: value.disable_loop_cover_traffic_stream,
        }
    }
}

impl Default for CoverTrafficV3 {
    fn default() -> Self {
        CoverTrafficV3 {
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            cover_traffic_primary_size_ratio: DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO,
            disable_loop_cover_traffic_stream: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GatewayConnectionV3 {
    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,
}

impl From<GatewayConnectionV3> for GatewayConnectionV4 {
    fn from(value: GatewayConnectionV3) -> Self {
        GatewayConnectionV4 {
            gateway_response_timeout: value.gateway_response_timeout,
        }
    }
}

impl Default for GatewayConnectionV3 {
    fn default() -> Self {
        GatewayConnectionV3 {
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AcknowledgementsV3 {
    #[serde(with = "humantime_serde")]
    pub average_ack_delay: Duration,
    pub ack_wait_multiplier: f64,
    #[serde(with = "humantime_serde")]
    pub ack_wait_addition: Duration,
}

impl From<AcknowledgementsV3> for AcknowledgementsV4 {
    fn from(value: AcknowledgementsV3) -> Self {
        AcknowledgementsV4 {
            average_ack_delay: value.average_ack_delay,
            ack_wait_multiplier: value.ack_wait_multiplier,
            ack_wait_addition: value.ack_wait_addition,
        }
    }
}

impl Default for AcknowledgementsV3 {
    fn default() -> Self {
        AcknowledgementsV3 {
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopologyV3 {
    #[serde(with = "humantime_serde")]
    pub topology_refresh_rate: Duration,
    #[serde(with = "humantime_serde")]
    pub topology_resolution_timeout: Duration,
    pub disable_refreshing: bool,
}

impl Default for TopologyV3 {
    fn default() -> Self {
        TopologyV3 {
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            disable_refreshing: false,
        }
    }
}

impl From<TopologyV3> for TopologyV4 {
    fn from(value: TopologyV3) -> Self {
        TopologyV4 {
            topology_refresh_rate: value.topology_refresh_rate,
            topology_resolution_timeout: value.topology_resolution_timeout,
            disable_refreshing: value.disable_refreshing,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReplySurbsV3 {
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

impl Default for ReplySurbsV3 {
    fn default() -> Self {
        ReplySurbsV3 {
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

impl From<ReplySurbsV3> for ReplySurbsV4 {
    fn from(value: ReplySurbsV3) -> Self {
        ReplySurbsV4 {
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

#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugConfigV3 {
    pub traffic: TrafficV3,
    pub cover_traffic: CoverTrafficV3,
    pub gateway_connection: GatewayConnectionV3,
    pub acknowledgements: AcknowledgementsV3,
    pub topology: TopologyV3,
    pub reply_surbs: ReplySurbsV3,
}

impl From<DebugConfigV3> for DebugConfigV4 {
    fn from(value: DebugConfigV3) -> Self {
        DebugConfigV4 {
            traffic: value.traffic.into(),
            cover_traffic: value.cover_traffic.into(),
            gateway_connection: value.gateway_connection.into(),
            acknowledgements: value.acknowledgements.into(),
            topology: value.topology.into(),
            reply_surbs: value.reply_surbs.into(),
        }
    }
}
