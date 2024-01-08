// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{
    AcknowledgementsWasm, CoverTrafficWasm, DebugWasm, GatewayConnectionWasm, ReplySurbsWasm,
    TopologyWasm, TrafficWasm,
};
use crate::config::ConfigDebug;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

// just a helper structure to more easily pass through the JS boundary
#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct DebugWasmOverride {
    /// Defines all configuration options related to traffic streams.
    #[tsify(optional)]
    pub traffic: Option<TrafficWasmOverride>,

    /// Defines all configuration options related to cover traffic stream(s).
    #[tsify(optional)]
    pub cover_traffic: Option<CoverTrafficWasmOverride>,

    /// Defines all configuration options related to the gateway connection.
    #[tsify(optional)]
    pub gateway_connection: Option<GatewayConnectionWasmOverride>,

    /// Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
    #[tsify(optional)]
    pub acknowledgements: Option<AcknowledgementsWasmOverride>,

    /// Defines all configuration options related topology, such as refresh rates or timeouts.
    #[tsify(optional)]
    pub topology: Option<TopologyWasmOverride>,

    /// Defines all configuration options related to reply SURBs.
    #[tsify(optional)]
    pub reply_surbs: Option<ReplySurbsWasmOverride>,
}

impl From<DebugWasmOverride> for DebugWasm {
    fn from(value: DebugWasmOverride) -> Self {
        DebugWasm {
            traffic: value.traffic.map(Into::into).unwrap_or_default(),
            cover_traffic: value.cover_traffic.map(Into::into).unwrap_or_default(),
            gateway_connection: value.gateway_connection.map(Into::into).unwrap_or_default(),
            acknowledgements: value.acknowledgements.map(Into::into).unwrap_or_default(),
            topology: value.topology.map(Into::into).unwrap_or_default(),
            reply_surbs: value.reply_surbs.map(Into::into).unwrap_or_default(),
        }
    }
}

impl From<DebugWasmOverride> for ConfigDebug {
    fn from(value: DebugWasmOverride) -> Self {
        let debug_wasm: DebugWasm = value.into();
        debug_wasm.into()
    }
}

#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct TrafficWasmOverride {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[tsify(optional)]
    pub average_packet_delay_ms: Option<u32>,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    #[tsify(optional)]
    pub message_sending_average_delay_ms: Option<u32>,

    /// Controls whether the main packet stream constantly produces packets according to the predefined
    /// poisson distribution.
    #[tsify(optional)]
    pub disable_main_poisson_packet_distribution: Option<bool>,

    /// Controls whether the sent sphinx packet use the NON-DEFAULT bigger size.
    #[tsify(optional)]
    pub use_extended_packet_size: Option<bool>,

    /// Controls whether the sent packets should use outfox as opposed to the default sphinx.
    #[tsify(optional)]
    pub use_outfox: Option<bool>,
}

impl From<TrafficWasmOverride> for TrafficWasm {
    fn from(value: TrafficWasmOverride) -> Self {
        let def = TrafficWasm::default();

        TrafficWasm {
            average_packet_delay_ms: value
                .average_packet_delay_ms
                .unwrap_or(def.average_packet_delay_ms),
            message_sending_average_delay_ms: value
                .message_sending_average_delay_ms
                .unwrap_or(def.message_sending_average_delay_ms),
            disable_main_poisson_packet_distribution: value
                .disable_main_poisson_packet_distribution
                .unwrap_or(def.disable_main_poisson_packet_distribution),
            use_extended_packet_size: value
                .use_extended_packet_size
                .unwrap_or(def.use_extended_packet_size),
            use_outfox: value.use_outfox.unwrap_or(def.use_outfox),
        }
    }
}

#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct CoverTrafficWasmOverride {
    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    #[tsify(optional)]
    pub loop_cover_traffic_average_delay_ms: Option<u32>,

    /// Specifies the ratio of `primary_packet_size` to `secondary_packet_size` used in cover traffic.
    /// Only applicable if `secondary_packet_size` is enabled.
    #[tsify(optional)]
    pub cover_traffic_primary_size_ratio: Option<f64>,

    /// Controls whether the dedicated loop cover traffic stream should be enabled.
    /// (and sending packets, on average, every [Self::loop_cover_traffic_average_delay])
    #[tsify(optional)]
    pub disable_loop_cover_traffic_stream: Option<bool>,
}

impl From<CoverTrafficWasmOverride> for CoverTrafficWasm {
    fn from(value: CoverTrafficWasmOverride) -> Self {
        let def = CoverTrafficWasm::default();

        CoverTrafficWasm {
            loop_cover_traffic_average_delay_ms: value
                .loop_cover_traffic_average_delay_ms
                .unwrap_or(def.loop_cover_traffic_average_delay_ms),
            cover_traffic_primary_size_ratio: value
                .cover_traffic_primary_size_ratio
                .unwrap_or(def.cover_traffic_primary_size_ratio),
            disable_loop_cover_traffic_stream: value
                .disable_loop_cover_traffic_stream
                .unwrap_or(def.disable_loop_cover_traffic_stream),
        }
    }
}

#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConnectionWasmOverride {
    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    #[tsify(optional)]
    pub gateway_response_timeout_ms: Option<u32>,
}

impl From<GatewayConnectionWasmOverride> for GatewayConnectionWasm {
    fn from(value: GatewayConnectionWasmOverride) -> Self {
        let def = GatewayConnectionWasm::default();

        GatewayConnectionWasm {
            gateway_response_timeout_ms: value
                .gateway_response_timeout_ms
                .unwrap_or(def.gateway_response_timeout_ms),
        }
    }
}

#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct AcknowledgementsWasmOverride {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent acknowledgement is going to be delayed at any given mix node.
    /// So for an ack going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    #[tsify(optional)]
    pub average_ack_delay_ms: Option<u32>,

    /// Value multiplied with the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 1.
    #[tsify(optional)]
    pub ack_wait_multiplier: Option<f64>,

    /// Value added to the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 0.
    #[tsify(optional)]
    pub ack_wait_addition_ms: Option<u32>,
}

impl From<AcknowledgementsWasmOverride> for AcknowledgementsWasm {
    fn from(value: AcknowledgementsWasmOverride) -> Self {
        let def = AcknowledgementsWasm::default();

        AcknowledgementsWasm {
            average_ack_delay_ms: value
                .average_ack_delay_ms
                .unwrap_or(def.average_ack_delay_ms),
            ack_wait_multiplier: value.ack_wait_multiplier.unwrap_or(def.ack_wait_multiplier),
            ack_wait_addition_ms: value
                .ack_wait_addition_ms
                .unwrap_or(def.ack_wait_addition_ms),
        }
    }
}

#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct TopologyWasmOverride {
    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    #[tsify(optional)]
    pub topology_refresh_rate_ms: Option<u32>,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    #[tsify(optional)]
    pub topology_resolution_timeout_ms: Option<u32>,

    /// Defines how long the client is going to wait on startup for its gateway to come online,
    /// before abandoning the procedure.
    #[tsify(optional)]
    pub max_startup_gateway_waiting_period_ms: Option<u32>,

    /// Specifies whether the client should not refresh the network topology after obtaining
    /// the first valid instance.
    /// Supersedes `topology_refresh_rate_ms`.
    #[tsify(optional)]
    pub disable_refreshing: Option<bool>,
}

impl From<TopologyWasmOverride> for TopologyWasm {
    fn from(value: TopologyWasmOverride) -> Self {
        let def = TopologyWasm::default();

        TopologyWasm {
            topology_refresh_rate_ms: value
                .topology_refresh_rate_ms
                .unwrap_or(def.topology_refresh_rate_ms),
            topology_resolution_timeout_ms: value
                .topology_resolution_timeout_ms
                .unwrap_or(def.topology_resolution_timeout_ms),
            max_startup_gateway_waiting_period_ms: value
                .max_startup_gateway_waiting_period_ms
                .unwrap_or(def.max_startup_gateway_waiting_period_ms),
            disable_refreshing: value.disable_refreshing.unwrap_or(def.disable_refreshing),
        }
    }
}

#[derive(Tsify, Debug, Copy, Clone, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct ReplySurbsWasmOverride {
    /// Defines the minimum number of reply surbs the client wants to keep in its storage at all times.
    /// It can only allow to go below that value if its to request additional reply surbs.
    #[tsify(optional)]
    pub minimum_reply_surb_storage_threshold: Option<usize>,

    /// Defines the maximum number of reply surbs the client wants to keep in its storage at any times.
    #[tsify(optional)]
    pub maximum_reply_surb_storage_threshold: Option<usize>,

    /// Defines the minimum number of reply surbs the client would request.
    #[tsify(optional)]
    pub minimum_reply_surb_request_size: Option<u32>,

    /// Defines the maximum number of reply surbs the client would request.
    #[tsify(optional)]
    pub maximum_reply_surb_request_size: Option<u32>,

    /// Defines the maximum number of reply surbs a remote party is allowed to request from this client at once.
    #[tsify(optional)]
    pub maximum_allowed_reply_surb_request_size: Option<u32>,

    /// Defines maximum amount of time the client is going to wait for reply surbs before explicitly asking
    /// for more even though in theory they wouldn't need to.
    #[tsify(optional)]
    pub maximum_reply_surb_rerequest_waiting_period_ms: Option<u32>,

    /// Defines maximum amount of time the client is going to wait for reply surbs before
    /// deciding it's never going to get them and would drop all pending messages
    #[tsify(optional)]
    pub maximum_reply_surb_drop_waiting_period_ms: Option<u32>,

    /// Defines maximum amount of time given reply surb is going to be valid for.
    /// This is going to be superseded by key rotation once implemented.
    #[tsify(optional)]
    pub maximum_reply_surb_age_ms: Option<u32>,

    /// Defines maximum amount of time given reply key is going to be valid for.
    /// This is going to be superseded by key rotation once implemented.
    #[tsify(optional)]
    pub maximum_reply_key_age_ms: Option<u32>,

    #[tsify(optional)]
    pub surb_mix_hops: Option<u8>,
}

impl From<ReplySurbsWasmOverride> for ReplySurbsWasm {
    fn from(value: ReplySurbsWasmOverride) -> Self {
        let def = ReplySurbsWasm::default();

        ReplySurbsWasm {
            minimum_reply_surb_storage_threshold: value
                .minimum_reply_surb_storage_threshold
                .unwrap_or(def.minimum_reply_surb_storage_threshold),
            maximum_reply_surb_storage_threshold: value
                .maximum_reply_surb_storage_threshold
                .unwrap_or(def.maximum_reply_surb_storage_threshold),
            minimum_reply_surb_request_size: value
                .minimum_reply_surb_request_size
                .unwrap_or(def.minimum_reply_surb_request_size),
            maximum_reply_surb_request_size: value
                .maximum_reply_surb_request_size
                .unwrap_or(def.maximum_reply_surb_request_size),
            maximum_allowed_reply_surb_request_size: value
                .maximum_allowed_reply_surb_request_size
                .unwrap_or(def.maximum_allowed_reply_surb_request_size),
            maximum_reply_surb_rerequest_waiting_period_ms: value
                .maximum_reply_surb_rerequest_waiting_period_ms
                .unwrap_or(def.maximum_reply_surb_rerequest_waiting_period_ms),
            maximum_reply_surb_drop_waiting_period_ms: value
                .maximum_reply_surb_drop_waiting_period_ms
                .unwrap_or(def.maximum_reply_surb_drop_waiting_period_ms),
            maximum_reply_surb_age_ms: value
                .maximum_reply_surb_age_ms
                .unwrap_or(def.maximum_reply_surb_age_ms),
            maximum_reply_key_age_ms: value
                .maximum_reply_key_age_ms
                .unwrap_or(def.maximum_reply_key_age_ms),
            surb_mix_hops: value.surb_mix_hops,
        }
    }
}
