// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to expansion of #[wasm_bindgen] macro on `Debug` Config struct
#![allow(clippy::drop_non_drop)]

use crate::error::WasmCoreError;
use nym_client_core::config::{ForgetMe, RememberMe};
use nym_config::helpers::OptionalSet;
use nym_sphinx::params::{PacketSize, PacketType};
use nym_statistics_common::types::SessionType;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use wasm_bindgen::prelude::*;

pub mod r#override;

pub use nym_client_core::config::{
    Acknowledgements as ConfigAcknowledgements, Config as BaseClientConfig,
    CoverTraffic as ConfigCoverTraffic, DebugConfig as ConfigDebug,
    GatewayConnection as ConfigGatewayConnection, ReplySurbs as ConfigReplySurbs,
    StatsReporting as ConfigStatsReporting, Topology as ConfigTopology, Traffic as ConfigTraffic,
};

pub fn new_base_client_config(
    id: String,
    version: String,
    nym_api: Option<String>,
    nyxd: Option<String>,
    debug: Option<DebugWasm>,
) -> Result<BaseClientConfig, WasmCoreError> {
    let nym_api_url = match nym_api {
        Some(raw) => Some(
            raw.parse()
                .map_err(|source| WasmCoreError::MalformedUrl { raw, source })?,
        ),
        None => None,
    };

    let nyxd_url = match nyxd {
        Some(raw) => Some(
            raw.parse()
                .map_err(|source| WasmCoreError::MalformedUrl { raw, source })?,
        ),
        None => None,
    };

    Ok(BaseClientConfig::new(id, version)
        .with_optional(
            BaseClientConfig::with_custom_nym_apis,
            nym_api_url.map(|u| vec![u]),
        )
        .with_optional(
            BaseClientConfig::with_custom_nyxd,
            nyxd_url.map(|u| vec![u]),
        )
        .with_debug_config(debug.map(Into::into).unwrap_or_default()))
}

#[wasm_bindgen]
pub fn default_debug() -> DebugWasm {
    ConfigDebug::default().into()
}

#[wasm_bindgen(js_name = defaultDebug)]
pub fn default_debug_obj() -> JsValue {
    let dbg = default_debug();
    serde_wasm_bindgen::to_value(&dbg)
        .expect("our 'DebugWasm' struct should have had a correctly defined serde implementation")
}

/// A client configuration in which no cover traffic will be sent,
/// in either the main distribution or the secondary traffic stream.
#[wasm_bindgen]
pub fn no_cover_debug() -> DebugWasm {
    let mut cfg = ConfigDebug::default();
    cfg.traffic.disable_main_poisson_packet_distribution = true;
    cfg.cover_traffic.disable_loop_cover_traffic_stream = true;
    cfg.into()
}

#[wasm_bindgen(js_name = noCoverDebug)]
pub fn no_cover_debug_obj() -> JsValue {
    let dbg = no_cover_debug();
    serde_wasm_bindgen::to_value(&dbg)
        .expect("our 'DebugWasm' struct should have had a correctly defined serde implementation")
}

// just a helper structure to more easily pass through the JS boundary
#[wasm_bindgen(inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugWasm {
    /// Defines all configuration options related to traffic streams.
    pub traffic: TrafficWasm,

    /// Defines all configuration options related to cover traffic stream(s).
    pub cover_traffic: CoverTrafficWasm,

    /// Defines all configuration options related to the gateway connection.
    pub gateway_connection: GatewayConnectionWasm,

    /// Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
    pub acknowledgements: AcknowledgementsWasm,

    /// Defines all configuration options related topology, such as refresh rates or timeouts.
    pub topology: TopologyWasm,

    /// Defines all configuration options related to reply SURBs.
    pub reply_surbs: ReplySurbsWasm,

    /// Defines all configuration options related to stats reporting.
    #[wasm_bindgen(getter_with_clone)]
    pub stats_reporting: StatsReportingWasm,

    pub forget_me: ForgetMeWasm,

    pub remember_me: RememberMeWasm,
}

impl Default for DebugWasm {
    fn default() -> Self {
        ConfigDebug::default().into()
    }
}

impl From<DebugWasm> for ConfigDebug {
    fn from(debug: DebugWasm) -> Self {
        ConfigDebug {
            traffic: debug.traffic.into(),
            cover_traffic: debug.cover_traffic.into(),
            gateway_connection: debug.gateway_connection.into(),
            acknowledgements: debug.acknowledgements.into(),
            topology: debug.topology.into(),
            reply_surbs: debug.reply_surbs.into(),
            stats_reporting: debug.stats_reporting.into(),
            forget_me: debug.forget_me.into(),
            remember_me: debug.remember_me.into(),
        }
    }
}

impl From<ConfigDebug> for DebugWasm {
    fn from(debug: ConfigDebug) -> Self {
        DebugWasm {
            traffic: debug.traffic.into(),
            cover_traffic: debug.cover_traffic.into(),
            gateway_connection: debug.gateway_connection.into(),
            acknowledgements: debug.acknowledgements.into(),
            topology: debug.topology.into(),
            reply_surbs: debug.reply_surbs.into(),
            stats_reporting: debug.stats_reporting.into(),
            forget_me: ForgetMeWasm::from(debug.forget_me),
            remember_me: RememberMeWasm::from(debug.remember_me),
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficWasm {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    pub average_packet_delay_ms: u32,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    pub message_sending_average_delay_ms: u32,

    /// Specify how many times particular packet can be retransmitted
    /// None - no limit
    pub maximum_number_of_retransmissions: Option<u32>,

    /// Specify whether route selection should be determined by the packet header.
    pub deterministic_route_selection: bool,

    /// Controls whether the main packet stream constantly produces packets according to the predefined
    /// poisson distribution.
    pub disable_main_poisson_packet_distribution: bool,

    /// Controls whether the sent sphinx packet use the NON-DEFAULT bigger size.
    pub use_extended_packet_size: bool,

    /// Specify whether any constructed sphinx packets should use the legacy format,
    /// where the payload keys are explicitly attached rather than using the seeds
    /// this affects any forward packets, acks and reply surbs
    /// this flag should remain disabled until sufficient number of nodes on the network has upgraded
    /// and support updated format.
    /// in the case of reply surbs, the recipient must also understand the new encoding
    pub use_legacy_sphinx_format: bool,

    /// Controls whether the sent packets should use outfox as opposed to the default sphinx.
    pub use_outfox: bool,

    /// Indicates whether to mix hops or not. If mix hops are enabled, traffic
    /// will be routed as usual, to the entry gateway, through three mix nodes, egressing
    /// through the exit gateway. If mix hops are disabled, traffic will be routed directly
    /// from the entry gateway to the exit gateway, bypassing the mix nodes.
    ///
    /// This overrides the `use_legacy_sphinx_format` setting as reduced/disabeld mix hops
    /// requires use of the updated SURB packet format.
    pub disable_mix_hops: bool,
}

impl Default for TrafficWasm {
    fn default() -> Self {
        ConfigTraffic::default().into()
    }
}

impl From<TrafficWasm> for ConfigTraffic {
    fn from(traffic: TrafficWasm) -> Self {
        let use_extended_packet_size = traffic
            .use_extended_packet_size
            .then_some(PacketSize::ExtendedPacket32);

        let packet_type = if traffic.use_outfox {
            PacketType::Outfox
        } else {
            PacketType::Mix
        };

        ConfigTraffic {
            average_packet_delay: Duration::from_millis(traffic.average_packet_delay_ms as u64),
            message_sending_average_delay: Duration::from_millis(
                traffic.message_sending_average_delay_ms as u64,
            ),
            deterministic_route_selection: traffic.deterministic_route_selection,
            maximum_number_of_retransmissions: traffic.maximum_number_of_retransmissions,
            disable_main_poisson_packet_distribution: traffic
                .disable_main_poisson_packet_distribution,
            primary_packet_size: PacketSize::RegularPacket,
            secondary_packet_size: use_extended_packet_size,
            use_legacy_sphinx_format: traffic.use_legacy_sphinx_format,
            packet_type,
            disable_mix_hops: traffic.disable_mix_hops,
        }
    }
}

impl From<ConfigTraffic> for TrafficWasm {
    fn from(traffic: ConfigTraffic) -> Self {
        TrafficWasm {
            average_packet_delay_ms: traffic.average_packet_delay.as_millis() as u32,
            message_sending_average_delay_ms: traffic.message_sending_average_delay.as_millis()
                as u32,
            deterministic_route_selection: traffic.deterministic_route_selection,
            maximum_number_of_retransmissions: traffic.maximum_number_of_retransmissions,
            disable_main_poisson_packet_distribution: traffic
                .disable_main_poisson_packet_distribution,
            use_legacy_sphinx_format: traffic.use_legacy_sphinx_format,
            use_extended_packet_size: traffic.secondary_packet_size.is_some(),
            use_outfox: traffic.packet_type == PacketType::Outfox,
            disable_mix_hops: traffic.disable_mix_hops,
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverTrafficWasm {
    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    pub loop_cover_traffic_average_delay_ms: u32,

    /// Specifies the ratio of `primary_packet_size` to `secondary_packet_size` used in cover traffic.
    /// Only applicable if `secondary_packet_size` is enabled.
    pub cover_traffic_primary_size_ratio: f64,

    /// Controls whether the dedicated loop cover traffic stream should be enabled.
    /// (and sending packets, on average, every [Self::loop_cover_traffic_average_delay])
    pub disable_loop_cover_traffic_stream: bool,
}

impl Default for CoverTrafficWasm {
    fn default() -> Self {
        ConfigCoverTraffic::default().into()
    }
}

impl From<CoverTrafficWasm> for ConfigCoverTraffic {
    fn from(cover_traffic: CoverTrafficWasm) -> Self {
        ConfigCoverTraffic {
            loop_cover_traffic_average_delay: Duration::from_millis(
                cover_traffic.loop_cover_traffic_average_delay_ms as u64,
            ),
            cover_traffic_primary_size_ratio: cover_traffic.cover_traffic_primary_size_ratio,
            disable_loop_cover_traffic_stream: cover_traffic.disable_loop_cover_traffic_stream,
        }
    }
}

impl From<ConfigCoverTraffic> for CoverTrafficWasm {
    fn from(cover_traffic: ConfigCoverTraffic) -> Self {
        CoverTrafficWasm {
            loop_cover_traffic_average_delay_ms: cover_traffic
                .loop_cover_traffic_average_delay
                .as_millis() as u32,
            cover_traffic_primary_size_ratio: cover_traffic.cover_traffic_primary_size_ratio,
            disable_loop_cover_traffic_stream: cover_traffic.disable_loop_cover_traffic_stream,
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayConnectionWasm {
    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    pub gateway_response_timeout_ms: u32,
}

impl Default for GatewayConnectionWasm {
    fn default() -> Self {
        ConfigGatewayConnection::default().into()
    }
}

impl From<GatewayConnectionWasm> for ConfigGatewayConnection {
    fn from(gateway_connection: GatewayConnectionWasm) -> Self {
        ConfigGatewayConnection {
            gateway_response_timeout: Duration::from_millis(
                gateway_connection.gateway_response_timeout_ms as u64,
            ),
        }
    }
}

impl From<ConfigGatewayConnection> for GatewayConnectionWasm {
    fn from(gateway_connection: ConfigGatewayConnection) -> Self {
        GatewayConnectionWasm {
            gateway_response_timeout_ms: gateway_connection.gateway_response_timeout.as_millis()
                as u32,
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcknowledgementsWasm {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent acknowledgement is going to be delayed at any given mix node.
    /// So for an ack going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    pub average_ack_delay_ms: u32,

    /// Value multiplied with the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 1.
    pub ack_wait_multiplier: f64,

    /// Value added to the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 0.
    pub ack_wait_addition_ms: u32,
}

impl Default for AcknowledgementsWasm {
    fn default() -> Self {
        ConfigAcknowledgements::default().into()
    }
}

impl From<AcknowledgementsWasm> for ConfigAcknowledgements {
    fn from(acknowledgements: AcknowledgementsWasm) -> Self {
        ConfigAcknowledgements {
            average_ack_delay: Duration::from_millis(acknowledgements.average_ack_delay_ms as u64),
            ack_wait_multiplier: acknowledgements.ack_wait_multiplier,
            ack_wait_addition: Duration::from_millis(acknowledgements.ack_wait_addition_ms as u64),
        }
    }
}

impl From<ConfigAcknowledgements> for AcknowledgementsWasm {
    fn from(acknowledgements: ConfigAcknowledgements) -> Self {
        AcknowledgementsWasm {
            average_ack_delay_ms: acknowledgements.average_ack_delay.as_millis() as u32,
            ack_wait_multiplier: acknowledgements.ack_wait_multiplier,
            ack_wait_addition_ms: acknowledgements.ack_wait_addition.as_millis() as u32,
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyWasm {
    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    pub topology_refresh_rate_ms: u32,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    pub topology_resolution_timeout_ms: u32,

    /// Defines how long the client is going to wait on startup for its gateway to come online,
    /// before abandoning the procedure.
    pub max_startup_gateway_waiting_period_ms: u32,

    /// Specifies whether the client should not refresh the network topology after obtaining
    /// the first valid instance.
    /// Supersedes `topology_refresh_rate_ms`.
    pub disable_refreshing: bool,

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

impl Default for TopologyWasm {
    fn default() -> Self {
        ConfigTopology::default().into()
    }
}

impl From<TopologyWasm> for ConfigTopology {
    fn from(topology: TopologyWasm) -> Self {
        ConfigTopology {
            topology_refresh_rate: Duration::from_millis(topology.topology_refresh_rate_ms as u64),
            topology_resolution_timeout: Duration::from_millis(
                topology.topology_resolution_timeout_ms as u64,
            ),
            disable_refreshing: topology.disable_refreshing,
            max_startup_gateway_waiting_period: Duration::from_millis(
                topology.max_startup_gateway_waiting_period_ms as u64,
            ),
            minimum_mixnode_performance: topology.minimum_mixnode_performance,
            minimum_gateway_performance: topology.minimum_gateway_performance,
            use_extended_topology: topology.use_extended_topology,
            ignore_egress_epoch_role: topology.ignore_egress_epoch_role,
            ignore_ingress_epoch_role: topology.ignore_ingress_epoch_role,
        }
    }
}

impl From<ConfigTopology> for TopologyWasm {
    fn from(topology: ConfigTopology) -> Self {
        TopologyWasm {
            topology_refresh_rate_ms: topology.topology_refresh_rate.as_millis() as u32,
            topology_resolution_timeout_ms: topology.topology_resolution_timeout.as_millis() as u32,
            max_startup_gateway_waiting_period_ms: topology
                .max_startup_gateway_waiting_period
                .as_millis() as u32,
            disable_refreshing: topology.disable_refreshing,
            minimum_mixnode_performance: topology.minimum_mixnode_performance,
            minimum_gateway_performance: topology.minimum_gateway_performance,
            use_extended_topology: topology.use_extended_topology,
            ignore_egress_epoch_role: topology.ignore_egress_epoch_role,
            ignore_ingress_epoch_role: topology.ignore_ingress_epoch_role,
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplySurbsWasm {
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
    pub maximum_reply_surb_rerequest_waiting_period_ms: u32,

    /// Defines maximum amount of time the client is going to wait for reply surbs before
    /// deciding it's never going to get them and would drop all pending messages
    pub maximum_reply_surb_drop_waiting_period_ms: u32,

    /// Defines maximum number of times the client is going to re-request reply surbs
    /// for clearing pending messages before giving up after making no progress.
    pub maximum_reply_surbs_rerequests: usize,

    /// Defines maximum amount of time given reply key is going to be valid for.
    /// This is going to be superseded by key rotation once implemented.
    pub maximum_reply_key_age_ms: u32,

    /// Defines how many mix nodes the reply surb should go through.
    /// If not set, the default value is going to be used.
    pub surb_mix_hops: Option<u8>,
}

impl Default for ReplySurbsWasm {
    fn default() -> Self {
        ConfigReplySurbs::default().into()
    }
}

impl From<ReplySurbsWasm> for ConfigReplySurbs {
    fn from(reply_surbs: ReplySurbsWasm) -> Self {
        ConfigReplySurbs {
            minimum_reply_surb_storage_threshold: reply_surbs.minimum_reply_surb_storage_threshold,
            maximum_reply_surb_storage_threshold: reply_surbs.maximum_reply_surb_storage_threshold,
            minimum_reply_surb_threshold_buffer: reply_surbs.minimum_reply_surb_threshold_buffer,
            minimum_reply_surb_request_size: reply_surbs.minimum_reply_surb_request_size,
            maximum_reply_surb_request_size: reply_surbs.maximum_reply_surb_request_size,
            maximum_allowed_reply_surb_request_size: reply_surbs
                .maximum_allowed_reply_surb_request_size,
            maximum_reply_surb_rerequest_waiting_period: Duration::from_millis(
                reply_surbs.maximum_reply_surb_rerequest_waiting_period_ms as u64,
            ),
            maximum_reply_surb_drop_waiting_period: Duration::from_millis(
                reply_surbs.maximum_reply_surb_drop_waiting_period_ms as u64,
            ),
            maximum_reply_surbs_rerequests: reply_surbs.maximum_reply_surbs_rerequests,
            maximum_reply_key_age: Duration::from_millis(
                reply_surbs.maximum_reply_key_age_ms as u64,
            ),
            surb_mix_hops: reply_surbs.surb_mix_hops,
        }
    }
}

impl From<ConfigReplySurbs> for ReplySurbsWasm {
    fn from(reply_surbs: ConfigReplySurbs) -> Self {
        ReplySurbsWasm {
            minimum_reply_surb_storage_threshold: reply_surbs.minimum_reply_surb_storage_threshold,
            maximum_reply_surb_storage_threshold: reply_surbs.maximum_reply_surb_storage_threshold,
            minimum_reply_surb_threshold_buffer: reply_surbs.minimum_reply_surb_threshold_buffer,
            minimum_reply_surb_request_size: reply_surbs.minimum_reply_surb_request_size,
            maximum_reply_surb_request_size: reply_surbs.maximum_reply_surb_request_size,
            maximum_allowed_reply_surb_request_size: reply_surbs
                .maximum_allowed_reply_surb_request_size,
            maximum_reply_surb_rerequest_waiting_period_ms: reply_surbs
                .maximum_reply_surb_rerequest_waiting_period
                .as_millis() as u32,
            maximum_reply_surb_drop_waiting_period_ms: reply_surbs
                .maximum_reply_surb_drop_waiting_period
                .as_millis() as u32,
            maximum_reply_surbs_rerequests: reply_surbs.maximum_reply_surbs_rerequests,
            maximum_reply_key_age_ms: reply_surbs.maximum_reply_key_age.as_millis() as u32,
            surb_mix_hops: reply_surbs.surb_mix_hops,
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Clone, Deserialize, PartialEq, Serialize, Copy, Default)]
#[serde(deny_unknown_fields)]
pub struct ForgetMeWasm {
    pub client: bool,
    pub stats: bool,
}

impl From<ForgetMeWasm> for ForgetMe {
    fn from(value: ForgetMeWasm) -> Self {
        ForgetMe::new(value.client, value.stats)
    }
}

impl From<ForgetMe> for ForgetMeWasm {
    fn from(value: ForgetMe) -> Self {
        Self {
            client: value.client(),
            stats: value.stats(),
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RememberMeWasm {
    pub stats: bool,
}

impl From<RememberMeWasm> for RememberMe {
    fn from(value: RememberMeWasm) -> Self {
        RememberMe::new(value.stats, SessionType::Wasm)
    }
}

impl From<RememberMe> for RememberMeWasm {
    fn from(value: RememberMe) -> Self {
        Self {
            stats: value.stats(),
        }
    }
}

#[wasm_bindgen(inspectable)]
#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StatsReportingWasm {
    /// Is stats reporting enabled
    pub enabled: bool,

    /// Address of the stats collector. If this is none, no reporting will happen, regardless of `enabled`
    #[wasm_bindgen(getter_with_clone)]
    pub provider_address: Option<String>,

    /// With what frequence will statistics be sent
    pub reporting_interval_ms: u32,
}

impl Default for StatsReportingWasm {
    fn default() -> Self {
        ConfigStatsReporting::default().into()
    }
}

impl From<StatsReportingWasm> for ConfigStatsReporting {
    fn from(stats_reporting: StatsReportingWasm) -> Self {
        ConfigStatsReporting {
            enabled: stats_reporting.enabled,
            provider_address: stats_reporting
                .provider_address
                .map(|address| address.parse().expect("Invalid provider address")),
            reporting_interval: Duration::from_millis(stats_reporting.reporting_interval_ms as u64),
        }
    }
}

impl From<ConfigStatsReporting> for StatsReportingWasm {
    fn from(stats_reporting: ConfigStatsReporting) -> Self {
        StatsReportingWasm {
            enabled: stats_reporting.enabled,
            provider_address: stats_reporting.provider_address.map(|r| r.to_string()),
            reporting_interval_ms: stats_reporting.reporting_interval.as_millis() as u32,
        }
    }
}
