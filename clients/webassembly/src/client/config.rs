// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to expansion of #[wasm_bindgen] macro on `Debug` Config struct
#![allow(clippy::drop_non_drop)]

use client_core::config::{Debug as ConfigDebug, GatewayEndpoint};
use std::time::Duration;
use url::Url;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Config {
    /// ID specifies the human readable ID of this particular client.
    pub(crate) id: String,

    pub(crate) validator_api_url: Url,

    pub(crate) disabled_credentials_mode: bool,

    /// Information regarding how the client should send data to gateway.
    pub(crate) gateway_endpoint: GatewayEndpoint,

    pub(crate) debug: ConfigDebug,
}

#[wasm_bindgen]
impl Config {
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: String,
        validator_server: String,
        gateway_endpoint: GatewayEndpoint,
        debug: Option<Debug>,
    ) -> Self {
        Config {
            id,
            validator_api_url: validator_server
                .parse()
                .expect("provided url was malformed"),
            disabled_credentials_mode: true,
            gateway_endpoint,
            debug: debug.map(Into::into).unwrap_or_default(),
        }
    }
}

// just a helper structure to more easily pass through the JS boundary
#[wasm_bindgen]
pub struct Debug {
    /// The parameter of Poisson distribution determining how long, on average,
    /// sent packet is going to be delayed at any given mix node.
    /// So for a packet going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    pub average_packet_delay_ms: u64,

    /// The parameter of Poisson distribution determining how long, on average,
    /// sent acknowledgement is going to be delayed at any given mix node.
    /// So for an ack going through three mix nodes, on average, it will take three times this value
    /// until the packet reaches its destination.
    pub average_ack_delay_ms: u64,

    /// Value multiplied with the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 1.
    pub ack_wait_multiplier: f64,

    /// Value added to the expected round trip time of an acknowledgement packet before
    /// it is assumed it was lost and retransmission of the data packet happens.
    /// In an ideal network with 0 latency, this value would have been 0.
    pub ack_wait_addition_ms: u64,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take for another loop cover traffic message to be sent.
    pub loop_cover_traffic_average_delay_ms: u64,

    /// The parameter of Poisson distribution determining how long, on average,
    /// it is going to take another 'real traffic stream' message to be sent.
    /// If no real packets are available and cover traffic is enabled,
    /// a loop cover message is sent instead in order to preserve the rate.
    pub message_sending_average_delay_ms: u64,

    /// How long we're willing to wait for a response to a message sent to the gateway,
    /// before giving up on it.
    pub gateway_response_timeout_ms: u64,

    /// The uniform delay every which clients are querying the directory server
    /// to try to obtain a compatible network topology to send sphinx packets through.
    pub topology_refresh_rate_ms: u64,

    /// During topology refresh, test packets are sent through every single possible network
    /// path. This timeout determines waiting period until it is decided that the packet
    /// did not reach its destination.
    pub topology_resolution_timeout_ms: u64,

    /// Controls whether the dedicated loop cover traffic stream should be enabled.
    /// (and sending packets, on average, every [Self::loop_cover_traffic_average_delay_ms])
    pub disable_loop_cover_traffic_stream: bool,

    /// Controls whether the main packet stream constantly produces packets according to the predefined
    /// poisson distribution.
    pub disable_main_poisson_packet_distribution: bool,

    /// Controls whether the sent sphinx packet use the NON-DEFAULT bigger size.
    pub use_extended_packet_size: bool,
}

impl From<Debug> for ConfigDebug {
    fn from(debug: Debug) -> Self {
        // For now we just always use the (older) 32kb extended size
        let use_extended_packet_size = debug
            .use_extended_packet_size
            .then(|| String::from("extended32"));

        ConfigDebug {
            average_packet_delay: Duration::from_millis(debug.average_packet_delay_ms),
            average_ack_delay: Duration::from_millis(debug.average_ack_delay_ms),
            ack_wait_multiplier: debug.ack_wait_multiplier,
            ack_wait_addition: Duration::from_millis(debug.ack_wait_addition_ms),
            loop_cover_traffic_average_delay: Duration::from_millis(
                debug.loop_cover_traffic_average_delay_ms,
            ),
            message_sending_average_delay: Duration::from_millis(
                debug.message_sending_average_delay_ms,
            ),
            gateway_response_timeout: Duration::from_millis(debug.gateway_response_timeout_ms),
            topology_refresh_rate: Duration::from_millis(debug.topology_refresh_rate_ms),
            topology_resolution_timeout: Duration::from_millis(
                debug.topology_resolution_timeout_ms,
            ),
            disable_loop_cover_traffic_stream: debug.disable_loop_cover_traffic_stream,
            disable_main_poisson_packet_distribution: debug
                .disable_main_poisson_packet_distribution,
            use_extended_packet_size,
        }
    }
}

impl From<ConfigDebug> for Debug {
    fn from(debug: ConfigDebug) -> Self {
        Debug {
            average_packet_delay_ms: debug.average_packet_delay.as_millis() as u64,
            average_ack_delay_ms: debug.average_ack_delay.as_millis() as u64,
            ack_wait_multiplier: debug.ack_wait_multiplier,
            ack_wait_addition_ms: debug.ack_wait_addition.as_millis() as u64,
            loop_cover_traffic_average_delay_ms: debug.loop_cover_traffic_average_delay.as_millis()
                as u64,
            message_sending_average_delay_ms: debug.message_sending_average_delay.as_millis()
                as u64,
            gateway_response_timeout_ms: debug.gateway_response_timeout.as_millis() as u64,
            topology_refresh_rate_ms: debug.topology_refresh_rate.as_millis() as u64,
            topology_resolution_timeout_ms: debug.topology_resolution_timeout.as_millis() as u64,
            disable_loop_cover_traffic_stream: debug.disable_loop_cover_traffic_stream,
            disable_main_poisson_packet_distribution: debug
                .disable_main_poisson_packet_distribution,
            use_extended_packet_size: debug.use_extended_packet_size.is_some(),
        }
    }
}

#[wasm_bindgen]
pub fn default_debug() -> Debug {
    ConfigDebug::default().into()
}
