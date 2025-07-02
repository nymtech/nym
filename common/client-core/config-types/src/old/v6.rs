// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Acknowledgements, Client, Config, CoverTraffic, DebugConfig, ForgetMe, GatewayConnection,
    RememberMe, ReplySurbs, StatsReporting, Topology, Traffic,
};
use nym_config::serde_helpers::{de_maybe_stringified, ser_maybe_stringified};
use nym_sphinx_addressing::Recipient;
use nym_sphinx_params::{PacketSize, PacketType};
use nym_statistics_common::types::SessionType;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

// 'DEBUG'
const DEFAULT_ACK_WAIT_MULTIPLIER: f64 = 1.5;

const DEFAULT_ACK_WAIT_ADDITION: Duration = Duration::from_millis(1_500);
const DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(200);
const DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY: Duration = Duration::from_millis(20);
const DEFAULT_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(15);
const DEFAULT_TOPOLOGY_REFRESH_RATE: Duration = Duration::from_secs(5 * 60); // every 5min
const DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT: Duration = Duration::from_millis(5_000);

// the same values as our current (10.06.24) blacklist
const DEFAULT_MIN_MIXNODE_PERFORMANCE: u8 = 50;
const DEFAULT_MIN_GATEWAY_PERFORMANCE: u8 = 50;

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
const DEFAULT_MINIMUM_REPLY_SURB_THRESHOLD_BUFFER: usize = 0;

// define how much to request at once
// clients/client-core/src/client/replies/reply_controller.rs
const DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE: u32 = 10;
const DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE: u32 = 50;

const DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE: u32 = 500;

const DEFAULT_MAXIMUM_REPLY_SURB_REREQUEST_WAITING_PERIOD: Duration = Duration::from_secs(10);
const DEFAULT_MAXIMUM_REPLY_SURB_DROP_WAITING_PERIOD: Duration = Duration::from_secs(5 * 60);

// 12 hours
const DEFAULT_MAXIMUM_REPLY_SURB_AGE: Duration = Duration::from_secs(12 * 60 * 60);

// 24 hours
const DEFAULT_MAXIMUM_REPLY_KEY_AGE: Duration = Duration::from_secs(24 * 60 * 60);

// stats reporting related

/// Time interval between reporting statistics to the given provider if it exists
const STATS_REPORT_INTERVAL_SECS: Duration = Duration::from_secs(300);

// aliases for backwards compatibility
pub type ConfigV1_1_54 = ConfigV6;
pub type ClientV1_1_54 = ClientV6;
pub type DebugConfigV1_1_54 = DebugConfigV6;

pub type TrafficV1_1_54 = TrafficV6;
pub type CoverTrafficV1_1_54 = CoverTrafficV6;
pub type GatewayConnectionV1_1_54 = GatewayConnectionV6;
pub type AcknowledgementsV1_1_54 = AcknowledgementsV6;
pub type TopologyV1_1_54 = TopologyV6;
pub type ReplySurbsV1_1_54 = ReplySurbsV6;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV6 {
    pub client: ClientV6,

    #[serde(default)]
    pub debug: DebugConfigV6,
}

impl From<ConfigV6> for Config {
    fn from(value: ConfigV6) -> Self {
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
                    average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
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
                    deterministic_route_selection: value
                        .debug
                        .traffic
                        .deterministic_route_selection,
                    maximum_number_of_retransmissions: value
                        .debug
                        .traffic
                        .maximum_number_of_retransmissions,
                    use_legacy_sphinx_format: value.debug.traffic.use_legacy_sphinx_format,
                    disable_mix_hops: value.debug.traffic.disable_mix_hops,
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
                    minimum_mixnode_performance: value.debug.topology.minimum_mixnode_performance,
                    minimum_gateway_performance: value.debug.topology.minimum_gateway_performance,
                    use_extended_topology: value.debug.topology.use_extended_topology,
                    ignore_egress_epoch_role: value.debug.topology.ignore_egress_epoch_role,
                    ignore_ingress_epoch_role: value.debug.topology.ignore_ingress_epoch_role,
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
                    maximum_reply_key_age: value.debug.reply_surbs.maximum_reply_key_age,
                    surb_mix_hops: value.debug.reply_surbs.surb_mix_hops,
                    minimum_reply_surb_threshold_buffer: value
                        .debug
                        .reply_surbs
                        .minimum_reply_surb_threshold_buffer,
                    ..Default::default()
                },
                stats_reporting: StatsReporting {
                    enabled: value.debug.stats_reporting.enabled,
                    provider_address: value.debug.stats_reporting.provider_address,
                    reporting_interval: value.debug.stats_reporting.reporting_interval,
                },
                forget_me: ForgetMe {
                    client: value.debug.forget_me.client,
                    stats: value.debug.forget_me.stats,
                },
                remember_me: RememberMe {
                    stats: value.debug.remember_me.stats,
                    session_type: value.debug.remember_me.session_type.into(),
                },
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
// note: the deny_unknown_fields is VITAL here to allow upgrades from v1.1.20_2
#[serde(deny_unknown_fields)]
pub struct ClientV6 {
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
pub struct TrafficV6 {
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

    /// Specify whether route selection should be determined by the packet header.
    pub deterministic_route_selection: bool,

    /// Specify how many times particular packet can be retransmitted
    /// None - no limit
    pub maximum_number_of_retransmissions: Option<u32>,

    /// Specifies the packet size used for sent messages.
    /// Do not override it unless you understand the consequences of that change.
    pub primary_packet_size: PacketSize,

    /// Specifies the optional auxiliary packet size for optimizing message streams.
    /// Note that its use decreases overall anonymity.
    /// Do not set it unless you understand the consequences of that change.
    pub secondary_packet_size: Option<PacketSize>,

    /// Specify whether any constructed sphinx packets should use the legacy format,
    /// where the payload keys are explicitly attached rather than using the seeds
    /// this affects any forward packets, acks and reply surbs
    /// this flag should remain disabled until sufficient number of nodes on the network has upgraded
    /// and support updated format.
    /// in the case of reply surbs, the recipient must also understand the new encoding
    pub use_legacy_sphinx_format: bool,

    pub packet_type: PacketType,

    /// Indicates whether to mix hops or not. If mix hops are enabled, traffic
    /// will be routed as usual, to the entry gateway, through three mix nodes, egressing
    /// through the exit gateway. If mix hops are disabled, traffic will be routed directly
    /// from the entry gateway to the exit gateway, bypassing the mix nodes.
    pub disable_mix_hops: bool,
}

impl Default for TrafficV6 {
    fn default() -> Self {
        TrafficV6 {
            average_packet_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            message_sending_average_delay: DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY,
            disable_main_poisson_packet_distribution: false,
            deterministic_route_selection: false,
            maximum_number_of_retransmissions: None,
            primary_packet_size: PacketSize::RegularPacket,
            secondary_packet_size: None,
            packet_type: PacketType::Mix,

            // we should use the legacy format until sufficient number of nodes understand the
            // improved encoding
            use_legacy_sphinx_format: true,
            disable_mix_hops: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CoverTrafficV6 {
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

impl Default for CoverTrafficV6 {
    fn default() -> Self {
        CoverTrafficV6 {
            loop_cover_traffic_average_delay: DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY,
            cover_traffic_primary_size_ratio: DEFAULT_COVER_TRAFFIC_PRIMARY_SIZE_RATIO,
            disable_loop_cover_traffic_stream: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GatewayConnectionV6 {
    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    #[serde(with = "humantime_serde")]
    pub gateway_response_timeout: Duration,
}

impl Default for GatewayConnectionV6 {
    fn default() -> Self {
        GatewayConnectionV6 {
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AcknowledgementsV6 {
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

impl Default for AcknowledgementsV6 {
    fn default() -> Self {
        AcknowledgementsV6 {
            average_ack_delay: DEFAULT_AVERAGE_PACKET_DELAY,
            ack_wait_multiplier: DEFAULT_ACK_WAIT_MULTIPLIER,
            ack_wait_addition: DEFAULT_ACK_WAIT_ADDITION,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopologyV6 {
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

    /// Specifies a minimum performance of a mixnode that is used on route construction.
    /// This setting is only applicable when `NymApi` topology is used.
    pub minimum_mixnode_performance: u8,

    /// Specifies a minimum performance of a gateway that is used on route construction.
    /// This setting is only applicable when `NymApi` topology is used.
    pub minimum_gateway_performance: u8,

    /// Specifies whether this client should attempt to retrieve all available network nodes
    /// as opposed to just active mixnodes/gateways.
    pub use_extended_topology: bool,

    /// Specifies whether this client should ignore the current epoch role of the target egress node
    /// when constructing the final hop packets.
    pub ignore_egress_epoch_role: bool,

    /// Specifies whether this client should ignore the current epoch role of the ingress node
    /// when attempting to establish new connection
    pub ignore_ingress_epoch_role: bool,
}

impl Default for TopologyV6 {
    fn default() -> Self {
        TopologyV6 {
            topology_refresh_rate: DEFAULT_TOPOLOGY_REFRESH_RATE,
            topology_resolution_timeout: DEFAULT_TOPOLOGY_RESOLUTION_TIMEOUT,
            disable_refreshing: false,
            max_startup_gateway_waiting_period: DEFAULT_MAX_STARTUP_GATEWAY_WAITING_PERIOD,
            minimum_mixnode_performance: DEFAULT_MIN_MIXNODE_PERFORMANCE,
            minimum_gateway_performance: DEFAULT_MIN_GATEWAY_PERFORMANCE,
            use_extended_topology: false,

            ignore_egress_epoch_role: true,
            ignore_ingress_epoch_role: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReplySurbsV6 {
    /// Defines the minimum number of reply surbs the client wants to keep in its storage at all times.
    /// It can only allow to go below that value if its to request additional reply surbs.
    pub minimum_reply_surb_storage_threshold: usize,

    /// Defines the maximum number of reply surbs the client wants to keep in its storage at any times.
    pub maximum_reply_surb_storage_threshold: usize,

    /// Defines the soft threshold ontop of the minimum reply surb storage threshold for when the client
    /// should proactively request additional reply surbs.
    pub minimum_reply_surb_threshold_buffer: usize,

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

    /// Specifies if we should reset all the sender tags on startup
    pub fresh_sender_tags: bool,
}

impl Default for ReplySurbsV6 {
    fn default() -> Self {
        ReplySurbsV6 {
            minimum_reply_surb_storage_threshold: DEFAULT_MINIMUM_REPLY_SURB_STORAGE_THRESHOLD,
            maximum_reply_surb_storage_threshold: DEFAULT_MAXIMUM_REPLY_SURB_STORAGE_THRESHOLD,
            minimum_reply_surb_threshold_buffer: DEFAULT_MINIMUM_REPLY_SURB_THRESHOLD_BUFFER,
            minimum_reply_surb_request_size: DEFAULT_MINIMUM_REPLY_SURB_REQUEST_SIZE,
            maximum_reply_surb_request_size: DEFAULT_MAXIMUM_REPLY_SURB_REQUEST_SIZE,
            maximum_allowed_reply_surb_request_size: DEFAULT_MAXIMUM_ALLOWED_SURB_REQUEST_SIZE,
            maximum_reply_surb_rerequest_waiting_period:
                DEFAULT_MAXIMUM_REPLY_SURB_REREQUEST_WAITING_PERIOD,
            maximum_reply_surb_drop_waiting_period: DEFAULT_MAXIMUM_REPLY_SURB_DROP_WAITING_PERIOD,
            maximum_reply_surb_age: DEFAULT_MAXIMUM_REPLY_SURB_AGE,
            maximum_reply_key_age: DEFAULT_MAXIMUM_REPLY_KEY_AGE,
            surb_mix_hops: None,
            fresh_sender_tags: false,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugConfigV6 {
    /// Defines all configuration options related to traffic streams.
    pub traffic: TrafficV6,

    /// Defines all configuration options related to cover traffic stream(s).
    pub cover_traffic: CoverTrafficV6,

    /// Defines all configuration options related to the gateway connection.
    pub gateway_connection: GatewayConnectionV6,

    /// Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
    pub acknowledgements: AcknowledgementsV6,

    /// Defines all configuration options related topology, such as refresh rates or timeouts.
    pub topology: TopologyV6,

    /// Defines all configuration options related to reply SURBs.
    pub reply_surbs: ReplySurbsV6,

    /// Defines all configuration options related to stats reporting.
    pub stats_reporting: StatsReportingV6,

    /// Defines all configuration options related to the forget me flag.
    pub forget_me: ForgetMeV6,

    /// Defines all configuration options related to the remember me flag.
    pub remember_me: RememberMeV6,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatsReportingV6 {
    /// Is stats reporting enabled
    pub enabled: bool,

    /// Address of the stats collector. If this is none, no reporting will happen, regardless of `enabled`
    #[serde(
        serialize_with = "ser_maybe_stringified",
        deserialize_with = "de_maybe_stringified"
    )]
    pub provider_address: Option<Recipient>,

    /// With what frequence will statistics be sent
    #[serde(with = "humantime_serde")]
    pub reporting_interval: Duration,
}

impl Default for StatsReportingV6 {
    fn default() -> Self {
        StatsReportingV6 {
            enabled: true,
            provider_address: None,
            reporting_interval: STATS_REPORT_INTERVAL_SECS,
        }
    }
}

#[derive(Clone, Default, Debug, Deserialize, PartialEq, Serialize, Copy)]
pub struct ForgetMeV6 {
    client: bool,
    stats: bool,
}

#[derive(Clone, Default, Debug, Deserialize, PartialEq, Serialize, Copy)]
pub struct RememberMeV6 {
    /// Signal that this client should be accounted for in the stats
    stats: bool,

    /// Type of the session to remember, if it should be remembered
    session_type: SessionTypeV6,
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize, Default, Debug)]
pub enum SessionTypeV6 {
    Vpn,
    Mixnet,
    Wasm,
    Native,
    Socks5,
    #[default]
    Unknown,
}

impl From<SessionTypeV6> for SessionType {
    fn from(value: SessionTypeV6) -> Self {
        match value {
            SessionTypeV6::Vpn => Self::Vpn,
            SessionTypeV6::Mixnet => Self::Mixnet,
            SessionTypeV6::Wasm => Self::Wasm,
            SessionTypeV6::Native => Self::Native,
            SessionTypeV6::Socks5 => Self::Socks5,
            SessionTypeV6::Unknown => Self::Unknown,
        }
    }
}
