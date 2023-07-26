import type { DebugWasm } from './types-from-wasm-pack';

export * from './types-from-wasm-pack';

/**
 * Some common mime types, however, you can always just specify the mime-type as a string. Test
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

/**
 * @ignore
 * @internal
 */
export type OnPayloadFn = (payload: Payload) => void;
/**
 * @ignore
 * @internal
 */
export type OnRawPayloadFn = (payload: Uint8Array) => void;
/**
 * @ignore
 * @internal
 */
export type EventHandlerFn<E> = (e: E) => void | Promise<void>;
/**
 * @ignore
 * @internal
 */
export type EventHandlerSubscribeFn<E> = (fn: EventHandlerFn<E>) => EventHandlerUnsubscribeFn;
/**
 * @ignore
 * @internal
 */
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
  debug?: DebugWasm;
}

/**
 *
 * @ignore
 * @hidden
 * @internal
 */
export interface IWebWorker {
  start: (config: NymClientConfig) => void;
  stop: () => void;
  selfAddress: () => string | undefined;
  setTextMimeTypes: (mimeTypes: string[]) => void;
  getTextMimeTypes: () => string[];
  send: (args: { payload: Payload; recipient: string; replySurbs?: number }) => void;
  rawSend: (args: { payload: Uint8Array; recipient: string; replySurbs?: number }) => void;
}

export interface Client {
  /**
   * Start the client.
   *
   * @example
   *
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   *  clientId: 'my-client',
   *  nymApiUrl: 'https://validator.nymtech.net',
   * });
   *
   */
  start: (config: NymClientConfig) => Promise<void>;

  stop: () => Promise<void>;
  /**
    Get the client address
   */
  selfAddress: () => Promise<string | undefined>;

  setTextMimeTypes: (mimeTypes: string[]) => void;
  getTextMimeTypes: () => Promise<string[]>;
  send: (args: { payload: Payload; recipient: string; replySurbs?: number }) => Promise<void>;
  /**
   * Send a raw payload, without any mime-type conversion.
   */
  rawSend: (args: { payload: Uint8Array; recipient: string; replySurbs?: number }) => Promise<void>;
}

  /**
 * Enum representing various event kinds.
 * @enum
 */
export enum EventKinds {
    /**
   * The event emitted when the nodetester is ready to be used.
   */
  Loaded = 'Loaded',

     /**
   * The event emitted when connection to the gateway is established.
   */
  Connected = 'Connected',
   
  /**
   * The event for when a message is received and interpreted as a string.
   */
  StringMessageReceived = 'StringMessageReceived',

    /**
   * The event for when a binary message is received. BinaryMessage is a type of message that contains additional metadata, such as MIME type and some headers, along with the actual payload data.
   */
  BinaryMessageReceived = 'BinaryMessageReceived',

    /**
   * The event for when a raw message is received. RawMessage represents the bytes that are received directly from the mixnet with no further parsing or interpretation done on them.
   */
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

export interface Events {
  subscribeToLoaded: EventHandlerSubscribeFn<LoadedEvent>;
  subscribeToConnected: EventHandlerSubscribeFn<ConnectedEvent>;
  subscribeToTextMessageReceivedEvent: EventHandlerSubscribeFn<StringMessageReceivedEvent>;
  subscribeToBinaryMessageReceivedEvent: EventHandlerSubscribeFn<BinaryMessageReceivedEvent>;
  subscribeToRawMessageReceivedEvent: EventHandlerSubscribeFn<RawMessageReceivedEvent>;
}
