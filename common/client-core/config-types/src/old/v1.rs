// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::old::v2::{
    AcknowledgementsV2, ClientV2, ConfigV2, CoverTrafficV2, DebugConfigV2, GatewayConnectionV2,
    LoggingV2, ReplySurbsV2, TopologyV2, TrafficV2, DEFAULT_ACK_WAIT_ADDITION,
    DEFAULT_ACK_WAIT_MULTIPLIER, DEFAULT_AVERAGE_PACKET_DELAY, DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
    DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY, DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE,
    DEFAULT_MAXIMUM_REPLY_KEY_AGE, DEFAULT_MAXIMUM_REPLY_SURB_AGE,
    DEFAULT_MAXIMUM_REPLY_SURB_DROP_WAITING_PERIOD, DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE,
    DEFAULT_MAXIMUM_REPLY_SURB_REREQUEST_WAITING_PERIOD,
    DEFAULT_MAXIMUM_REPLY_SURB_STORAGE_THRESHOLD, DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
    DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE, DEFAULT_MINIMUM_REPLY_SURB_STORAGE_THRESHOLD,
    DEFAULT_TOPOLOGY_REFRESH_RATE, DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
};
use nym_sphinx_params::PacketSize;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::Duration;

// aliases for backwards compatibility
pub type OldConfigV1_1_13<T> = ConfigV1<T>;
pub type OldLoggingV1_1_13 = LoggingV1;
pub type OldDebugConfigV1_1_13 = DebugConfigV1;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExtendedPacketSize {
    Extended8,
    Extended16,
    Extended32,
}

impl From<ExtendedPacketSize> for PacketSize {
    fn from(size: ExtendedPacketSize) -> PacketSize {
        match size {
            ExtendedPacketSize::Extended8 => PacketSize::ExtendedPacket8,
            ExtendedPacketSize::Extended16 => PacketSize::ExtendedPacket16,
            ExtendedPacketSize::Extended32 => PacketSize::ExtendedPacket32,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1<T> {
    pub client: ClientV2<T>,

    #[serde(default)]
    pub logging: LoggingV1,
    #[serde(default)]
    pub debug: DebugConfigV1,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingV1 {}

impl From<LoggingV1> for LoggingV2 {
    fn from(_value: LoggingV1) -> Self {
        LoggingV2 {}
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugConfigV1 {
    #[serde(with = "humantime_serde")]
    pub average_packet_delay: Duration,

    #[serde(with = "humantime_serde")]
    pub average_ack_delay: Duration,

    pub ack_wait_multiplier: f64,
    #[serde(with = "humantime_serde")]
    pub ack_wait_addition: Duration,

    #[serde(with = "humantime_serde")]
    pub loop_cover_traffic_average_delay: Duration,

    #[serde(with = "humantime_serde")]
    pub message_sending_average_delay: Duration,

    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,

    #[serde(with = "humantime_serde")]
    pub topology_refresh_rate: Duration,

    #[serde(with = "humantime_serde")]
    pub topology_resolution_timeout: Duration,

    pub disable_loop_cover_traffic_stream: bool,

    pub disable_main_poisson_packet_distribution: bool,

    pub use_extended_packet_size: Option<ExtendedPacketSize>,

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

impl From<DebugConfigV1> for DebugConfigV2 {
    fn from(value: DebugConfigV1) -> Self {
        DebugConfigV2 {
            traffic: TrafficV2 {
                average_packet_delay: value.average_packet_delay,
                message_sending_average_delay: value.message_sending_average_delay,
                disable_main_poisson_packet_distribution: value
                    .disable_main_poisson_packet_distribution,
                primary_packet_size: PacketSize::RegularPacket,
                secondary_packet_size: value.use_extended_packet_size.map(Into::into),
            },
            cover_traffic: CoverTrafficV2 {
                loop_cover_traffic_average_delay: value.loop_cover_traffic_average_delay,
                disable_loop_cover_traffic_stream: value.disable_loop_cover_traffic_stream,
                ..CoverTrafficV2::default()
            },
            gateway_connection: GatewayConnectionV2 {
                gateway_response_timeout: value.gateway_response_timeout,
            },
            acknowledgements: AcknowledgementsV2 {
                average_ack_delay: value.average_ack_delay,
                ack_wait_multiplier: value.ack_wait_multiplier,
                ack_wait_addition: value.ack_wait_addition,
            },
            topology: TopologyV2 {
                topology_refresh_rate: value.topology_refresh_rate,
                topology_resolution_timeout: value.topology_resolution_timeout,
                disable_refreshing: false,
            },
            reply_surbs: ReplySurbsV2 {
                minimum_reply_surb_storage_threshold: value.minimum_reply_surb_storage_threshold,
                maximum_reply_surb_storage_threshold: value.maximum_reply_surb_storage_threshold,
                minimum_reply_surb_request_size: value.minimum_reply_surb_request_size,
                maximum_reply_surb_request_size: value.maximum_reply_surb_request_size,
                maximum_allowed_reply_surb_request_size: value
                    .maximum_allowed_reply_surb_request_size,
                maximum_reply_surb_rerequest_waiting_period: value
                    .maximum_reply_surb_rerequest_waiting_period,
                maximum_reply_surb_drop_waiting_period: value
                    .maximum_reply_surb_drop_waiting_period,
                maximum_reply_surb_age: value.maximum_reply_surb_age,
                maximum_reply_key_age: value.maximum_reply_key_age,
            },
        }
    }
}

impl Default for DebugConfigV1 {
    fn default() -> Self {
        DebugConfigV1 {
            average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            message_sending_average_delay: DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            disable_loop_cover_traffic_stream: false,
            disable_main_poisson_packet_distribution: false,
            use_extended_packet_size: None,
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

impl<T, U> From<ConfigV1<T>> for ConfigV2<U> {
    fn from(value: ConfigV1<T>) -> Self {
        ConfigV2 {
            client: ClientV2 {
                version: value.client.version,
                id: value.client.id,
                disabled_credentials_mode: value.client.disabled_credentials_mode,
                nyxd_urls: value.client.nyxd_urls,
                nym_api_urls: value.client.nym_api_urls,
                private_identity_key_file: value.client.private_identity_key_file,
                public_identity_key_file: value.client.public_identity_key_file,
                private_encryption_key_file: value.client.private_encryption_key_file,
                public_encryption_key_file: value.client.public_encryption_key_file,
                gateway_shared_key_file: value.client.gateway_shared_key_file,
                ack_key_file: value.client.ack_key_file,
                gateway_endpoint: value.client.gateway_endpoint,
                database_path: value.client.database_path,
                reply_surb_database_path: value.client.reply_surb_database_path,
                nym_root_directory: value.client.nym_root_directory,

                super_struct: PhantomData,
            },
            logging: value.logging.into(),
            debug: value.debug.into(),
        }
    }
}
