/**
 */
export interface TopologyWasm {
  free(): void;
  /**
   * Specifies whether the client should not refresh the network topology after obtaining
   * the first valid instance.
   * Supersedes `topology_refresh_rate_ms`.
   */
  disable_refreshing: boolean;
  /**
   * The uniform delay every which clients are querying the directory server
   * to try to obtain a compatible network topology to send sphinx packets through.
   */
  topology_refresh_rate_ms: bigint;
  /**
   * During topology refresh, test packets are sent through every single possible network
   * path. This timeout determines waiting period until it is decided that the packet
   * did not reach its destination.
   */
  topology_resolution_timeout_ms: bigint;
}
/**
 */
export interface TrafficWasm {
  free(): void;
  /**
   * The parameter of Poisson distribution determining how long, on average,
   * sent packet is going to be delayed at any given mix node.
   * So for a packet going through three mix nodes, on average, it will take three times this value
   * until the packet reaches its destination.
   */
  average_packet_delay_ms: bigint;
  /**
   * Controls whether the main packet stream constantly produces packets according to the predefined
   * poisson distribution.
   */
  disable_main_poisson_packet_distribution: boolean;
  /**
   * The parameter of Poisson distribution determining how long, on average,
   * it is going to take another 'real traffic stream' message to be sent.
   * If no real packets are available and cover traffic is enabled,
   * a loop cover message is sent instead in order to preserve the rate.
   */
  message_sending_average_delay_ms: bigint;
  /**
   * Controls whether the sent sphinx packet use the NON-DEFAULT bigger size.
   */
  use_extended_packet_size: boolean;
  /**
   * Controls whether the sent packets should use outfox as opposed to the default sphinx.
   */
  use_outfox: boolean;
}
/**
 */
export interface WasmGateway {
  free(): void;
  /**
   */
  clients_port: number;
  /**
   */
  host: string;
  /**
   */
  identity_key: string;
  /**
   */
  mix_port: number;
  /**
   */
  owner: string;
  /**
   */
  sphinx_key: string;
  /**
   */
  version: string;
}

/**
 */
export interface ReplySurbsWasm {
  free(): void;
  /**
   * Defines the maximum number of reply surbs a remote party is allowed to request from this client at once.
   */
  maximum_allowed_reply_surb_request_size: number;
  /**
   * Defines maximum amount of time given reply key is going to be valid for.
   * This is going to be superseded by key rotation once implemented.
   */
  maximum_reply_key_age_ms: bigint;
  /**
   * Defines maximum amount of time given reply surb is going to be valid for.
   * This is going to be superseded by key rotation once implemented.
   */
  maximum_reply_surb_age_ms: bigint;
  /**
   * Defines maximum amount of time the client is going to wait for reply surbs before
   * deciding it's never going to get them and would drop all pending messages
   */
  maximum_reply_surb_drop_waiting_period_ms: bigint;
  /**
   * Defines the maximum number of reply surbs the client would request.
   */
  maximum_reply_surb_request_size: number;
  /**
   * Defines maximum amount of time the client is going to wait for reply surbs before explicitly asking
   * for more even though in theory they wouldn't need to.
   */
  maximum_reply_surb_rerequest_waiting_period_ms: bigint;
  /**
   * Defines the maximum number of reply surbs the client wants to keep in its storage at any times.
   */
  maximum_reply_surb_storage_threshold: number;
  /**
   * Defines the minimum number of reply surbs the client would request.
   */
  minimum_reply_surb_request_size: number;
  /**
   * Defines the minimum number of reply surbs the client wants to keep in its storage at all times.
   * It can only allow to go below that value if its to request additional reply surbs.
   */
  minimum_reply_surb_storage_threshold: number;
}

/**
 */
export interface GatewayConnectionWasm {
  free(): void;
  /**
   * How long we're willing to wait for a response to a message sent to the gateway,
   * before giving up on it.
   */
  gateway_response_timeout_ms: bigint;
}

/**
 */
export interface AcknowledgementsWasm {
  free(): void;
  /**
   * Value added to the expected round trip time of an acknowledgement packet before
   * it is assumed it was lost and retransmission of the data packet happens.
   * In an ideal network with 0 latency, this value would have been 0.
   */
  ack_wait_addition_ms: bigint;
  /**
   * Value multiplied with the expected round trip time of an acknowledgement packet before
   * it is assumed it was lost and retransmission of the data packet happens.
   * In an ideal network with 0 latency, this value would have been 1.
   */
  ack_wait_multiplier: number;
  /**
   * The parameter of Poisson distribution determining how long, on average,
   * sent acknowledgement is going to be delayed at any given mix node.
   * So for an ack going through three mix nodes, on average, it will take three times this value
   * until the packet reaches its destination.
   */
  average_ack_delay_ms: bigint;
}

/**
 */
export interface CoverTrafficWasm {
  free(): void;
  /**
   * Specifies the ratio of `primary_packet_size` to `secondary_packet_size` used in cover traffic.
   * Only applicable if `secondary_packet_size` is enabled.
   */
  cover_traffic_primary_size_ratio: number;
  /**
   * Controls whether the dedicated loop cover traffic stream should be enabled.
   * (and sending packets, on average, every [Self::loop_cover_traffic_average_delay])
   */
  disable_loop_cover_traffic_stream: boolean;
  /**
   * The parameter of Poisson distribution determining how long, on average,
   * it is going to take for another loop cover traffic message to be sent.
   */
  loop_cover_traffic_average_delay_ms: bigint;
}

export interface Config {
  free(): void;
}

export interface DebugWasm {
  free(): void;
  /**
   * Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
   */
  acknowledgements: AcknowledgementsWasm;
  /**
   * Defines all configuration options related to cover traffic stream(s).
   */
  cover_traffic: CoverTrafficWasm;
  /**
   * Defines all configuration options related to the gateway connection.
   */
  gateway_connection: GatewayConnectionWasm;
  /**
   * Defines all configuration options related to reply SURBs.
   */
  reply_surbs: ReplySurbsWasm;
  /**
   * Defines all configuration options related topology, such as refresh rates or timeouts.
   */
  topology: TopologyWasm;
  /**
   * Defines all configuration options related to traffic streams.
   */
  traffic: TrafficWasm;
}
/**
 */
export interface GatewayEndpointConfig {
  free(): void;
  /**
   * gateway_id specifies ID of the gateway to which the client should send messages.
   * If initially omitted, a random gateway will be chosen from the available topology.
   */
  gateway_id: string;
  /**
   * Address of the gateway listener to which all client requests should be sent.
   */
  gateway_listener: string;
  /**
   * Address of the gateway owner to which the client should send messages.
   */
  gateway_owner: string;
}
/**
 */
export interface NymClient {
  free(): void;
  /**
   * @returns {string}
   */
  self_address(): string;
  /**
   * The simplest message variant where no additional information is attached.
   * You're simply sending your `data` to specified `recipient` without any tagging.
   *
   * Ends up with `NymMessage::Plain` variant
   * @param {Uint8Array} message
   * @param {string} recipient
   * @returns {Promise<any>}
   */
  send_regular_message(message: Uint8Array, recipient: string): Promise<any>;
  /**
   * Creates a message used for a duplex anonymous communication where the recipient
   * will never learn of our true identity. This is achieved by carefully sending `reply_surbs`.
   *
   * Note that if reply_surbs is set to zero then
   * this variant requires the client having sent some reply_surbs in the past
   * (and thus the recipient also knowing our sender tag).
   *
   * Ends up with `NymMessage::Repliable` variant
   * @param {Uint8Array} message
   * @param {string} recipient
   * @param {number} reply_surbs
   * @returns {Promise<any>}
   */
  send_anonymous_message(message: Uint8Array, recipient: string, reply_surbs: number): Promise<any>;
  /**
   * Attempt to use our internally received and stored `ReplySurb` to send the message back
   * to specified recipient whilst not knowing its full identity (or even gateway).
   *
   * Ends up with `NymMessage::Reply` variant
   * @param {Uint8Array} message
   * @param {string} recipient_tag
   * @returns {Promise<any>}
   */
  send_reply(message: Uint8Array, recipient_tag: string): Promise<any>;
}

export interface Topology {
  free(): void;
  /**
   * Specifies whether the client should not refresh the network topology after obtaining
   * the first valid instance.
   * Supersedes `topology_refresh_rate_ms`.
   */
  disable_refreshing: boolean;
  /**
   * The uniform delay every which clients are querying the directory server
   * to try to obtain a compatible network topology to send sphinx packets through.
   */
  topology_refresh_rate_ms: bigint;
  /**
   * During topology refresh, test packets are sent through every single possible network
   * path. This timeout determines waiting period until it is decided that the packet
   * did not reach its destination.
   */
  topology_resolution_timeout_ms: bigint;
}

export interface Traffic {
  free(): void;
  /**
   * The parameter of Poisson distribution determining how long, on average,
   * sent packet is going to be delayed at any given mix node.
   * So for a packet going through three mix nodes, on average, it will take three times this value
   * until the packet reaches its destination.
   */
  average_packet_delay_ms: bigint;
  /**
   * Controls whether the main packet stream constantly produces packets according to the predefined
   * poisson distribution.
   */
  disable_main_poisson_packet_distribution: boolean;
  /**
   * The parameter of Poisson distribution determining how long, on average,
   * it is going to take another 'real traffic stream' message to be sent.
   * If no real packets are available and cover traffic is enabled,
   * a loop cover message is sent instead in order to preserve the rate.
   */
  message_sending_average_delay_ms: bigint;
  /**
   * Controls whether the sent sphinx packet use the NON-DEFAULT bigger size.
   */
  use_extended_packet_size: boolean;
}

/**
 */
export interface NymClientBuilder {
  free(): void;
  /**
   * @returns {Promise<Promise<any>>}
   */
  start_client(): Promise<Promise<any>>;
}
