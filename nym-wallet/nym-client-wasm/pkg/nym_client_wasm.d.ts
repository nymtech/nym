/* tslint:disable */
/* eslint-disable */
/**
* @param {string} api_server
* @param {string | undefined} preferred
* @returns {Promise<GatewayEndpointConfig>}
*/
export function get_gateway(api_server: string, preferred?: string): Promise<GatewayEndpointConfig>;
/**
* @param {string} nym_api_url
* @returns {Promise<any>}
*/
export function current_network_topology(nym_api_url: string): Promise<any>;
/**
* Encode a payload
* @param {string} mime_type
* @param {Uint8Array} payload
* @returns {Uint8Array}
*/
export function encode_payload(mime_type: string, payload: Uint8Array): Uint8Array;
/**
* Create a new binary message with a user-specified `kind`, and `headers` as a string.
* @param {string} mime_type
* @param {Uint8Array} payload
* @param {string | undefined} headers
* @returns {Uint8Array}
*/
export function encode_payload_with_headers(mime_type: string, payload: Uint8Array, headers?: string): Uint8Array;
/**
* Parse the `kind` and byte array `payload` from a byte array
* @param {Uint8Array} message
* @returns {EncodedPayload}
*/
export function decode_payload(message: Uint8Array): EncodedPayload;
/**
* Try parse a UTF-8 string from an array of bytes
* @param {Uint8Array} payload
* @returns {string}
*/
export function parse_utf8_string(payload: Uint8Array): string;
/**
* Converts a UTF-8 string into an array of bytes
*
* This method is provided as a replacement for the mess of `atob`
* (https://developer.mozilla.org/en-US/docs/Web/API/atob) helpers provided by browsers and NodeJS.
*
* Feel free to use `atob` if you know you won't have problems with polyfills or encoding issues.
* @param {string} message
* @returns {Uint8Array}
*/
export function utf8_string_to_byte_array(message: string): Uint8Array;
/**
* @param {string} recipient
*/
export function validate_recipient(recipient: string): void;
/**
*/
export function set_panic_hook(): void;
/**
* @returns {DebugWasm}
*/
export function default_debug(): DebugWasm;

export interface EncodedPayload {
    mimeType: string,
    payload: Uint8Array;
    headers: string,
}


/**
*/
export class AcknowledgementsWasm {
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
export class AnonymousSenderTag {
  free(): void;
}
/**
*/
export class ClientStorage {
  free(): void;
/**
* @param {string} client_id
* @param {string} passphrase
*/
  constructor(client_id: string, passphrase: string);
/**
* @param {string} client_id
* @returns {Promise<any>}
*/
  static new_unencrypted(client_id: string): Promise<any>;
}
/**
*/
export class Config {
  free(): void;
/**
* @param {string} id
* @param {string} validator_server
* @param {DebugWasm | undefined} debug
*/
  constructor(id: string, validator_server: string, debug?: DebugWasm);
}
/**
*/
export class CoverTrafficWasm {
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
/**
*/
export class DebugWasm {
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
export class GatewayConnectionWasm {
  free(): void;
/**
* How long we're willing to wait for a response to a message sent to the gateway,
* before giving up on it.
*/
  gateway_response_timeout_ms: bigint;
}
/**
*/
export class GatewayEndpointConfig {
  free(): void;
/**
* @param {string} gateway_id
* @param {string} gateway_owner
* @param {string} gateway_listener
*/
  constructor(gateway_id: string, gateway_owner: string, gateway_listener: string);
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
export class NodeTestResult {
  free(): void;
/**
*/
  log_details(): void;
/**
* @returns {number}
*/
  score(): number;
/**
*/
  duplicate_acks: number;
/**
*/
  duplicate_packets: number;
/**
*/
  received_acks: number;
/**
*/
  received_packets: number;
/**
*/
  sent_packets: number;
}
/**
*/
export class NymClient {
  free(): void;
/**
* @param {Config} config
* @param {Function} on_message
* @param {string | undefined} preferred_gateway
* @param {string | undefined} storage_passphrase
*/
  constructor(config: Config, on_message: Function, preferred_gateway?: string, storage_passphrase?: string);
/**
* @returns {string}
*/
  self_address(): string;
/**
* @param {string} mixnode_identity
* @param {number | undefined} num_test_packets
* @returns {Promise<any>}
*/
  try_construct_test_packet_request(mixnode_identity: string, num_test_packets?: number): Promise<any>;
/**
* @param {WasmNymTopology} topology
* @returns {Promise<any>}
*/
  change_hardcoded_topology(topology: WasmNymTopology): Promise<any>;
/**
* @returns {Promise<any>}
*/
  current_network_topology(): Promise<any>;
/**
* Sends a test packet through the current network topology.
* It's the responsibility of the caller to ensure the correct topology has been injected and
* correct onmessage handlers have been setup.
* @param {NymClientTestRequest} request
* @returns {Promise<any>}
*/
  try_send_test_packets(request: NymClientTestRequest): Promise<any>;
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
/**
*/
export class NymClientBuilder {
  free(): void;
/**
* @param {Config} config
* @param {Function} on_message
* @param {string | undefined} preferred_gateway
* @param {string | undefined} storage_passphrase
*/
  constructor(config: Config, on_message: Function, preferred_gateway?: string, storage_passphrase?: string);
/**
* @param {WasmNymTopology} topology
* @param {Function} on_message
* @param {string | undefined} gateway
* @returns {NymClientBuilder}
*/
  static new_tester(topology: WasmNymTopology, on_message: Function, gateway?: string): NymClientBuilder;
/**
* @returns {Promise<any>}
*/
  start_client(): Promise<any>;
}
/**
*/
export class NymClientTestRequest {
  free(): void;
/**
* @returns {WasmNymTopology}
*/
  injectable_topology(): WasmNymTopology;
}
/**
*/
export class NymNodeTester {
  free(): void;
/**
* @param {WasmNymTopology} topology
* @param {string | undefined} id
* @param {string | undefined} gateway
*/
  constructor(topology: WasmNymTopology, id?: string, gateway?: string);
/**
* @param {string} api_url
* @param {string | undefined} id
* @param {string | undefined} gateway
* @returns {Promise<any>}
*/
  static new_with_api(api_url: string, id?: string, gateway?: string): Promise<any>;
/**
* @returns {Promise<any>}
*/
  disconnect_from_gateway(): Promise<any>;
/**
* @returns {Promise<any>}
*/
  reconnect_to_gateway(): Promise<any>;
/**
* @param {string} mixnode_identity
* @param {bigint | undefined} timeout_millis
* @param {number | undefined} num_test_packets
* @returns {Promise<any>}
*/
  test_node(mixnode_identity: string, timeout_millis?: bigint, num_test_packets?: number): Promise<any>;
}
/**
*/
export class NymNodeTesterBuilder {
  free(): void;
/**
* @param {WasmNymTopology} base_topology
* @param {string | undefined} id
* @param {string | undefined} gateway
*/
  constructor(base_topology: WasmNymTopology, id?: string, gateway?: string);
/**
* @param {string} api_url
* @param {string | undefined} id
* @param {string | undefined} gateway
* @returns {Promise<any>}
*/
  static new_with_api(api_url: string, id?: string, gateway?: string): Promise<any>;
/**
* @returns {Promise<any>}
*/
  setup_client(): Promise<any>;
}
/**
*/
export class ReplySurbsWasm {
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
export class TopologyWasm {
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
export class TrafficWasm {
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
export class WasmGateway {
  free(): void;
/**
* @param {string} owner
* @param {string} host
* @param {number} mix_port
* @param {number} clients_port
* @param {string} identity_key
* @param {string} sphinx_key
* @param {string} version
*/
  constructor(owner: string, host: string, mix_port: number, clients_port: number, identity_key: string, sphinx_key: string, version: string);
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
export class WasmMixNode {
  free(): void;
/**
* @param {number} mix_id
* @param {string} owner
* @param {string} host
* @param {number} mix_port
* @param {string} identity_key
* @param {string} sphinx_key
* @param {number} layer
* @param {string} version
*/
  constructor(mix_id: number, owner: string, host: string, mix_port: number, identity_key: string, sphinx_key: string, layer: number, version: string);
/**
*/
  host: string;
/**
*/
  identity_key: string;
/**
*/
  layer: number;
/**
*/
  mix_id: number;
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
export class WasmNymTopology {
  free(): void;
/**
* @param {any} mixnodes
* @param {any} gateways
*/
  constructor(mixnodes: any, gateways: any);
/**
*/
  print(): void;
}
