// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_30::{
    AcknowledgementsV1_1_30, ClientV1_1_30, ConfigV1_1_30, CoverTrafficV1_1_30, DebugConfigV1_1_30,
    GatewayConnectionV1_1_30, ReplySurbsV1_1_30, TopologyV1_1_30, TrafficV1_1_30,
};
use crate::config::old_config_v1_1_33::OldGatewayEndpointConfigV1_1_33;
use nym_sphinx::params::{PacketSize, PacketType};
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

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_20_2 {
    pub client: ClientV1_1_20_2,

    #[serde(default)]
    pub debug: DebugConfigV1_1_20_2,
}

impl From<ConfigV1_1_20_2> for ConfigV1_1_30 {
    fn from(value: ConfigV1_1_20_2) -> Self {
        ConfigV1_1_30 {
            client: value.client.into(),
            debug: value.debug.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct GatewayEndpointConfigV1_1_20_2 {
    /// gateway_id specifies ID of the gateway to which the client should send messages.
    /// If initially omitted, a random gateway will be chosen from the available topology.
    pub gateway_id: String,

    /// Address of the gateway owner to which the client should send messages.
    pub gateway_owner: String,

    /// Address of the gateway listener to which all client requests should be sent.
    pub gateway_listener: String,
}

impl From<GatewayEndpointConfigV1_1_20_2> for OldGatewayEndpointConfigV1_1_33 {
    fn from(value: GatewayEndpointConfigV1_1_20_2) -> Self {
        OldGatewayEndpointConfigV1_1_33 {
            gateway_id: value.gateway_id,
            gateway_owner: value.gateway_owner,
            gateway_listener: value.gateway_listener,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct ClientV1_1_20_2 {
    pub version: String,

    pub id: String,

    #[serde(default)]
    pub disabled_credentials_mode: bool,

    #[serde(alias = "validator_urls")]
    pub nyxd_urls: Vec<Url>,

    #[serde(alias = "validator_api_urls")]
    pub nym_api_urls: Vec<Url>,
    pub gateway_endpoint: GatewayEndpointConfigV1_1_20_2,
}

impl From<ClientV1_1_20_2> for ClientV1_1_30 {
    fn from(value: ClientV1_1_20_2) -> Self {
        ClientV1_1_30 {
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
pub struct TrafficV1_1_20_2 {
    #[serde(with = "humantime_serde")]
    pub average_packet_delay: Duration,
    #[serde(with = "humantime_serde")]
    pub message_sending_average_delay: Duration,
    pub disable_main_poisson_packet_distribution: bool,
    pub primary_packet_size: PacketSize,
    pub secondary_packet_size: Option<PacketSize>,
    pub packet_type: PacketType,
}

impl From<TrafficV1_1_20_2> for TrafficV1_1_30 {
    fn from(value: TrafficV1_1_20_2) -> Self {
        TrafficV1_1_30 {
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

impl Default for TrafficV1_1_20_2 {
    fn default() -> Self {
        TrafficV1_1_20_2 {
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
pub struct CoverTrafficV1_1_20_2 {
    #[serde(with = "humantime_serde")]
    pub loop_cover_traffic_average_delay: Duration,
    pub cover_traffic_primary_size_ratio: f64,
    pub disable_loop_cover_traffic_stream: bool,
}

impl From<CoverTrafficV1_1_20_2> for CoverTrafficV1_1_30 {
    fn from(value: CoverTrafficV1_1_20_2) -> Self {
        CoverTrafficV1_1_30 {
            loop_cover_traffic_average_delay: value.loop_cover_traffic_average_delay,
            cover_traffic_primary_size_ratio: value.cover_traffic_primary_size_ratio,
            disable_loop_cover_traffic_stream: value.disable_loop_cover_traffic_stream,
        }
    }
}

impl Default for CoverTrafficV1_1_20_2 {
    fn default() -> Self {
        CoverTrafficV1_1_20_2 {
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            cover_traffic_primary_size_ratio: DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO,
            disable_loop_cover_traffic_stream: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GatewayConnectionV1_1_20_2 {
    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,
}

impl From<GatewayConnectionV1_1_20_2> for GatewayConnectionV1_1_30 {
    fn from(value: GatewayConnectionV1_1_20_2) -> Self {
        GatewayConnectionV1_1_30 {
            gateway_response_timeout: value.gateway_response_timeout,
        }
    }
}

impl Default for GatewayConnectionV1_1_20_2 {
    fn default() -> Self {
        GatewayConnectionV1_1_20_2 {
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AcknowledgementsV1_1_20_2 {
    #[serde(with = "humantime_serde")]
    pub average_ack_delay: Duration,
    pub ack_wait_multiplier: f64,
    #[serde(with = "humantime_serde")]
    pub ack_wait_addition: Duration,
}

impl From<AcknowledgementsV1_1_20_2> for AcknowledgementsV1_1_30 {
    fn from(value: AcknowledgementsV1_1_20_2) -> Self {
        AcknowledgementsV1_1_30 {
            average_ack_delay: value.average_ack_delay,
            ack_wait_multiplier: value.ack_wait_multiplier,
            ack_wait_addition: value.ack_wait_addition,
        }
    }
}

impl Default for AcknowledgementsV1_1_20_2 {
    fn default() -> Self {
        AcknowledgementsV1_1_20_2 {
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopologyV1_1_20_2 {
    #[serde(with = "humantime_serde")]
    pub topology_refresh_rate: Duration,
    #[serde(with = "humantime_serde")]
    pub topology_resolution_timeout: Duration,
    pub disable_refreshing: bool,
}

impl Default for TopologyV1_1_20_2 {
    fn default() -> Self {
        TopologyV1_1_20_2 {
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            disable_refreshing: false,
        }
    }
}

impl From<TopologyV1_1_20_2> for TopologyV1_1_30 {
    fn from(value: TopologyV1_1_20_2) -> Self {
        TopologyV1_1_30 {
            topology_refresh_rate: value.topology_refresh_rate,
            topology_resolution_timeout: value.topology_resolution_timeout,
            disable_refreshing: value.disable_refreshing,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReplySurbsV1_1_20_2 {
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

impl Default for ReplySurbsV1_1_20_2 {
    fn default() -> Self {
        ReplySurbsV1_1_20_2 {
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

impl From<ReplySurbsV1_1_20_2> for ReplySurbsV1_1_30 {
    fn from(value: ReplySurbsV1_1_20_2) -> Self {
        ReplySurbsV1_1_30 {
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
pub struct DebugConfigV1_1_20_2 {
    pub traffic: TrafficV1_1_20_2,
    pub cover_traffic: CoverTrafficV1_1_20_2,
    pub gateway_connection: GatewayConnectionV1_1_20_2,
    pub acknowledgements: AcknowledgementsV1_1_20_2,
    pub topology: TopologyV1_1_20_2,
    pub reply_surbs: ReplySurbsV1_1_20_2,
}

impl From<DebugConfigV1_1_20_2> for DebugConfigV1_1_30 {
    fn from(value: DebugConfigV1_1_20_2) -> Self {
        DebugConfigV1_1_30 {
            traffic: value.traffic.into(),
            cover_traffic: value.cover_traffic.into(),
            gateway_connection: value.gateway_connection.into(),
            acknowledgements: value.acknowledgements.into(),
            topology: value.topology.into(),
            reply_surbs: value.reply_surbs.into(),
        }
    }
}
