export interface Acknowledgements {
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

export interface CoverTraffic {
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

export interface GatewayConnection {
  free(): void;
  /**
   * How long we're willing to wait for a response to a message sent to the gateway,
   * before giving up on it.
   */
  gateway_response_timeout_ms: bigint;
}

export interface ReplySurbs {
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

export interface Debug {
  free(): void;
  /**
   * Defines all configuration options related to acknowledgements, such as delays or wait timeouts.
   */
  acknowledgements: Acknowledgements;
  /**
   * Defines all configuration options related to cover traffic stream(s).
   */
  cover_traffic: CoverTraffic;
  /**
   * Defines all configuration options related to the gateway connection.
   */
  gateway_connection: GatewayConnection;
  /**
   * Defines all configuration options related to reply SURBs.
   */
  reply_surbs: ReplySurbs;
  /**
   * Defines all configuration options related topology, such as refresh rates or timeouts.
   */
  topology: Topology;
  /**
   * Defines all configuration options related to traffic streams.
   */
  traffic: Traffic;
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
 * Some common mime types, however, you can always just specify the mime-type as a string
 */
export enum MimeTypes {
  ApplicationOctetStream = 'application/octet-stream',
  TextPlain = 'text/plain',
  ApplicationJson = 'application/json',
}

export interface Payload {
  message: string | Uint8Array;

  mimeType?: MimeTypes | string;

  headers?: string;
}

export type OnPayloadFn = (payload: Payload) => void;

export type OnRawPayloadFn = (payload: Uint8Array) => void;

export type EventHandlerFn<E> = (e: E) => void | Promise<void>;

export type EventHandlerSubscribeFn<E> = (fn: EventHandlerFn<E>) => EventHandlerUnsubscribeFn;

export type EventHandlerUnsubscribeFn = () => void;

export interface NymClientConfig {
  /**
   * A human-readable id for the client.
   */
  clientId: string;

  /**
   * The URL of a validator API to query for the network topology.
   */
  nymApiUrl: string;

  /**
   * Optional. The identity key of the preferred gateway to connect to.
   */
  preferredGatewayIdentityKey?: string;

  /**
   * Optional. The listener websocket of the preferred gateway to connect to.
   */
  gatewayListener?: string;

  /**
   * Optional. Settings for the WASM client.
   */
  debug?: Debug;
}

export interface IWebWorker {
  start: (config: NymClientConfig) => void;
  stop: () => void;
  selfAddress: () => string | undefined;
  setTextMimeTypes: (mimeTypes: string[]) => void;
  getTextMimeTypes: () => string[];
  send: (args: { payload: Payload; recipient: string; replySurbs?: number }) => void;
  rawSend: (args: { payload: Uint8Array; recipient: string; replySurbs?: number }) => void;
}

export interface IWebWorkerAsync {
  start: (config: NymClientConfig) => Promise<void>;
  stop: () => Promise<void>;
  selfAddress: () => Promise<string | undefined>;
  setTextMimeTypes: (mimeTypes: string[]) => void;
  getTextMimeTypes: () => Promise<string[]>;
  send: (args: { payload: Payload; recipient: string; replySurbs?: number }) => Promise<void>;
  rawSend: (args: { payload: Uint8Array; recipient: string; replySurbs?: number }) => Promise<void>;
}

export enum EventKinds {
  Loaded = 'Loaded',
  Connected = 'Connected',
  StringMessageReceived = 'StringMessageReceived',
  BinaryMessageReceived = 'BinaryMessageReceived',
  RawMessageReceived = 'RawMessageReceived',
}

export interface LoadedEvent {
  kind: EventKinds.Loaded;
  args: {
    loaded: true;
  };
}

export interface ConnectedEvent {
  kind: EventKinds.Connected;
  args: {
    address?: string;
  };
}

export interface StringMessageReceivedEvent {
  kind: EventKinds.StringMessageReceived;
  args: {
    mimeType: MimeTypes;
    payload: string;
    payloadRaw: Uint8Array;
    headers?: string;
  };
}

export interface BinaryMessageReceivedEvent {
  kind: EventKinds.BinaryMessageReceived;
  args: {
    mimeType: MimeTypes;
    payload: Uint8Array;
    headers?: string;
  };
}

export interface RawMessageReceivedEvent {
  kind: EventKinds.RawMessageReceived;
  args: {
    payload: Uint8Array;
  };
}

export interface IWebWorkerEvents {
  subscribeToLoaded: EventHandlerSubscribeFn<LoadedEvent>;
  subscribeToConnected: EventHandlerSubscribeFn<ConnectedEvent>;
  subscribeToTextMessageReceivedEvent: EventHandlerSubscribeFn<StringMessageReceivedEvent>;
  subscribeToBinaryMessageReceivedEvent: EventHandlerSubscribeFn<BinaryMessageReceivedEvent>;
  subscribeToRawMessageReceivedEvent: EventHandlerSubscribeFn<RawMessageReceivedEvent>;
}
