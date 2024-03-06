// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::topology_control::geo_aware_provider::CountryGroup;
use crate::config::{
    Acknowledgements, Client, Config, CoverTraffic, DebugConfig, GatewayConnection, GroupBy,
    ReplySurbs, Topology, TopologyStructure, Traffic,
};
use nym_sphinx::{
    addressing::clients::Recipient,
    params::{PacketSize, PacketType},
};
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
const DEFAULT_MAX_STARTUP_GATEWAY_WAITING_PERIOD: Duration = Duration::from_secs(70 * 60); // 70min -> full epoch (1h) + a bit of overhead

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
pub struct ConfigV1_1_33 {
    pub client: ClientV1_1_33,

    #[serde(default)]
    pub debug: DebugConfigV1_1_33,
}

impl From<ConfigV1_1_33> for Config {
    fn from(value: ConfigV1_1_33) -> Self {
        Config {
            client: Client {
                version: value.client.version,
                id: value.client.id,
                disabled_credentials_mode: value.client.disabled_credentials_mode,
                nyxd_urls: value.client.nyxd_urls,
                nym_api_urls: value.client.nym_api_urls,
            },
            debug: DebugConfig {
                traffic: Traffic {
                    average_packet_delay: value.debug.traffic.average_packet_delay,
                    message_sending_average_delay: value
                        .debug
                        .traffic
                        .message_sending_average_delay,
                    disable_main_poisson_packet_distribution: value
                        .debug
                        .traffic
                        .disable_main_poisson_packet_distribution,
                    primary_packet_size: value.debug.traffic.primary_packet_size,
                    secondary_packet_size: value.debug.traffic.secondary_packet_size,
                    packet_type: value.debug.traffic.packet_type,
                },
                cover_traffic: CoverTraffic {
                    loop_cover_traffic_average_delay: value
                        .debug
                        .cover_traffic
                        .loop_cover_traffic_average_delay,
                    cover_traffic_primary_size_ratio: value
                        .debug
                        .cover_traffic
                        .cover_traffic_primary_size_ratio,
                    disable_loop_cover_traffic_stream: value
                        .debug
                        .cover_traffic
                        .disable_loop_cover_traffic_stream,
                },
                gateway_connection: GatewayConnection {
                    gateway_response_timeout: value
                        .debug
                        .gateway_connection
                        .gateway_response_timeout,
                },
                acknowledgements: Acknowledgements {
                    average_ack_delay: value.debug.acknowledgements.average_ack_delay,
                    ack_wait_multiplier: value.debug.acknowledgements.ack_wait_multiplier,
                    ack_wait_addition: value.debug.acknowledgements.ack_wait_addition,
                },
                topology: Topology {
                    topology_refresh_rate: value.debug.topology.topology_refresh_rate,
                    topology_resolution_timeout: value.debug.topology.topology_resolution_timeout,
                    disable_refreshing: value.debug.topology.disable_refreshing,
                    max_startup_gateway_waiting_period: value
                        .debug
                        .topology
                        .max_startup_gateway_waiting_period,
                    topology_structure: value.debug.topology.topology_structure.into(),
                },
                reply_surbs: ReplySurbs {
                    minimum_reply_surb_storage_threshold: value
                        .debug
                        .reply_surbs
                        .minimum_reply_surb_storage_threshold,
                    maximum_reply_surb_storage_threshold: value
                        .debug
                        .reply_surbs
                        .maximum_reply_surb_storage_threshold,
                    minimum_reply_surb_request_size: value
                        .debug
                        .reply_surbs
                        .minimum_reply_surb_request_size,
                    maximum_reply_surb_request_size: value
                        .debug
                        .reply_surbs
                        .maximum_reply_surb_request_size,
                    maximum_allowed_reply_surb_request_size: value
                        .debug
                        .reply_surbs
                        .maximum_allowed_reply_surb_request_size,
                    maximum_reply_surb_rerequest_waiting_period: value
                        .debug
                        .reply_surbs
                        .maximum_reply_surb_rerequest_waiting_period,
                    maximum_reply_surb_drop_waiting_period: value
                        .debug
                        .reply_surbs
                        .maximum_reply_surb_drop_waiting_period,
                    maximum_reply_surb_age: value.debug.reply_surbs.maximum_reply_surb_age,
                    maximum_reply_key_age: value.debug.reply_surbs.maximum_reply_key_age,
                    surb_mix_hops: value.debug.reply_surbs.surb_mix_hops,
                },
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
// note: the deny_unknown_fields is VITAL here to allow upgrades from v1.1.20_2
#[serde(deny_unknown_fields)]
pub struct ClientV1_1_33 {
    /// Version of the client for which this configuration was created.
    pub version: String,

    /// ID specifies the human readable ID of this particular client.
    pub id: String,

    /// Indicates whether this client is running in a disabled credentials mode, thus attempting
    /// to claim bandwidth without presenting bandwidth credentials.
    // TODO: this should be moved to `debug.gateway_connection`
    #[serde(default)]
    pub disabled_credentials_mode: bool,

    /// Addresses to nyxd validators via which the client can communicate with the chain.
    #[serde(alias = "validator_urls")]
    pub nyxd_urls: Vec<Url>,

    /// Addresses to APIs running on validator from which the client gets the view of the network.
    #[serde(alias = "validator_api_urls")]
    pub nym_api_urls: Vec<Url>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TrafficV1_1_33 {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(with = "humantime_serde")]
    pub average_packet_delay: Duration,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    #[serde(with = "humantime_serde")]
    pub message_sending_average_delay: Duration,

    /// Controls whether the main packet stream constantly produces packets according to the predefined
    /// poisson distribution.
    pub disable_main_poisson_packet_distribution: bool,

    /// Specifies the packet size used for sent messages.
    /// Do not override it unless you understand the consequences of that change.
    pub primary_packet_size: PacketSize,

    /// Specifies the optional auxiliary packet size for optimizing message streams.
    /// Note that its use decreases overall anonymity.
    /// Do not set it it unless you understand the consequences of that change.
    pub secondary_packet_size: Option<PacketSize>,

    pub packet_type: PacketType,
}

impl Default for TrafficV1_1_33 {
    fn default() -> Self {
        TrafficV1_1_33 {
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
pub struct CoverTrafficV1_1_33 {
    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    #[serde(with = "humantime_serde")]
    pub loop_cover_traffic_average_delay: Duration,

    /// Specifies the ratio of `primary_packet_size` to `secondary_packet_size` used in cover traffic.
    /// Only applicable if `secondary_packet_size` is enabled.
    pub cover_traffic_primary_size_ratio: f64,

    /// Controls whether the dedicated loop cover traffic stream should be enabled.
    /// (and sending packets, on average, every [Self::loop_cover_traffic_average_delay])
    pub disable_loop_cover_traffic_stream: bool,
}

impl Default for CoverTrafficV1_1_33 {
    fn default() -> Self {
        CoverTrafficV1_1_33 {
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            cover_traffic_primary_size_ratio: DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO,
            disable_loop_cover_traffic_stream: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GatewayConnectionV1_1_33 {
    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,
}

impl Default for GatewayConnectionV1_1_33 {
    fn default() -> Self {
        GatewayConnectionV1_1_33 {
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AcknowledgementsV1_1_33 {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent acknowledgement is going to be delayed at any given mix node.
    /// So for an ack going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[serde(with = "humantime_serde")]
    pub average_ack_delay: Duration,

    /// Value multiplied with the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 1.
    pub ack_wait_multiplier: f64,

    /// Value added to the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 0.
    #[serde(with = "humantime_serde")]
    pub ack_wait_addition: Duration,
}

impl Default for AcknowledgementsV1_1_33 {
    fn default() -> Self {
        AcknowledgementsV1_1_33 {
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopologyV1_1_33 {
    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    #[serde(with = "humantime_serde")]
    pub topology_refresh_rate: Duration,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    #[serde(with = "humantime_serde")]
    pub topology_resolution_timeout: Duration,

    /// Specifies whether the client should not refresh the network topology after obtaining
    /// the first valid instance.
    /// Supersedes `topology_refresh_rate_ms`.
    pub disable_refreshing: bool,

    /// Defines how long the client is going to wait on startup for its gateway to come online,
    /// before abandoning the procedure.
    #[serde(with = "humantime_serde")]
    pub max_startup_gateway_waiting_period: Duration,

    /// Specifies the mixnode topology to be used for sending packets.
    pub topology_structure: TopologyStructureV1_1_33,
}

#[allow(clippy::large_enum_variant)]
#[derive(Default, Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TopologyStructureV1_1_33 {
    #[default]
    NymApi,
    GeoAware(GroupByV1_1_33),
}

impl From<TopologyStructureV1_1_33> for TopologyStructure {
    fn from(value: TopologyStructureV1_1_33) -> Self {
        match value {
            TopologyStructureV1_1_33::NymApi => TopologyStructure::NymApi,
            TopologyStructureV1_1_33::GeoAware(group_by) => {
                TopologyStructure::GeoAware(group_by.into())
            }
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GroupByV1_1_33 {
    CountryGroup(CountryGroup),
    NymAddress(Recipient),
}

impl From<GroupByV1_1_33> for GroupBy {
    fn from(value: GroupByV1_1_33) -> Self {
        match value {
            GroupByV1_1_33::CountryGroup(country) => GroupBy::CountryGroup(country),
            GroupByV1_1_33::NymAddress(addr) => GroupBy::NymAddress(addr),
        }
    }
}

impl std::fmt::Display for GroupByV1_1_33 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupByV1_1_33::CountryGroup(group) => write!(f, "group: {}", group),
            GroupByV1_1_33::NymAddress(address) => write!(f, "address: {}", address),
        }
    }
}

impl Default for TopologyV1_1_33 {
    fn default() -> Self {
        TopologyV1_1_33 {
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            disable_refreshing: false,
            max_startup_gateway_waiting_period: DEFAULT_MAX_STARTUP_GATEWAY_WAITING_PERIOD,
            topology_structure: TopologyStructureV1_1_33::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReplySurbsV1_1_33 {
    /// Defines the minimum number of reply surbs the client wants to keep in its storage at all times.
    /// It can only allow to go below that value if its to request additional reply surbs.
    pub minimum_reply_surb_storage_threshold: usize,

    /// Defines the maximum number of reply surbs the client wants to keep in its storage at any times.
    pub maximum_reply_surb_storage_threshold: usize,

    /// Defines the minimum number of reply surbs the client would request.
    pub minimum_reply_surb_request_size: u32,

    /// Defines the maximum number of reply surbs the client would request.
    pub maximum_reply_surb_request_size: u32,

    /// Defines the maximum number of reply surbs a remote party is allowed to request from this client at once.
    pub maximum_allowed_reply_surb_request_size: u32,

    /// Defines maximum amount of time the client is going to wait for reply surbs before explicitly asking
    /// for more even though in theory they wouldn't need to.
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_rerequest_waiting_period: Duration,

    /// Defines maximum amount of time the client is going to wait for reply surbs before
    /// deciding it's never going to get them and would drop all pending messages
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_drop_waiting_period: Duration,

    /// Defines maximum amount of time given reply surb is going to be valid for.
    /// This is going to be superseded by key rotation once implemented.
    #[serde(with = "humantime_serde")]
    pub maximum_reply_surb_age: Duration,

    /// Defines maximum amount of time given reply key is going to be valid for.
    /// This is going to be superseded by key rotation once implemented.
    #[serde(with = "humantime_serde")]
    pub maximum_reply_key_age: Duration,

    /// Specifies the number of mixnet hops the packet should go through. If not specified, then
    /// the default value is used.
    pub surb_mix_hops: Option<u8>,
}

impl Default for ReplySurbsV1_1_33 {
    fn default() -> Self {
        ReplySurbsV1_1_33 {
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
            surb_mix_hops: None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugConfigV1_1_33 {
    /// Defines all configuration options related to traffic streams.
    pub traffic: TrafficV1_1_33,

    /// Defines all configuration options related to cover traffic stream(s).
    pub cover_traffic: CoverTrafficV1_1_33,

    /// Defines all configuration options related to the gateway connection.
    pub gateway_connection: GatewayConnectionV1_1_33,

    /// Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
    pub acknowledgements: AcknowledgementsV1_1_33,

    /// Defines all configuration options related topology, such as refresh rates or timeouts.
    pub topology: TopologyV1_1_33,

    /// Defines all configuration options related to reply SURBs.
    pub reply_surbs: ReplySurbsV1_1_33,
}
