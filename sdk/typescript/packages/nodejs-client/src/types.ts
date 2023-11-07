import type { DebugWasm } from '@nymproject/nym-client-wasm-node';

/**
 * Options for the Nym mixnet client.
 * @property autoConvertStringMimeTypes - An array of mime types.
 * @example
 * ```typescript
 * const client = await createNymMixnetClient({
 *  autoConvertStringMimeTypes: [MimeTypes.ApplicationJson, MimeTypes.TextPlain],
 * });
 * ```
 */
export interface NymMixnetClientOptions {
  autoConvertStringMimeTypes?: string[] | MimeTypes[];
}

/**
 * The client for the Nym mixnet which gives access to client methods and event subscriptions.
 * Returned by the {@link createNymMixnetClient} function.
 * @property client - The sphinx nym wasm client.
 * @property events - Different streams of events provided by the client.
 */
export interface NymMixnetClient {
  client: Client;
  events: Events;
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
   *  nymApiUrl: 'https://validator.nymtech.net/api',
   * });
   *
   */
  start: (config: NymClientConfig) => Promise<void>;
  /**
   * Stop the client.
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   *  clientId: 'my-client',
   *  nymApiUrl: 'https://validator.nymtech.net/api',
   * });
   * await client.stop();
   * ```
   */
  stop: () => Promise<void>;
  /**
   * Get the client address
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   *  clientId: 'my-client',
   *  nymApiUrl: 'https://validator.nymtech.net/api',
   * });
   * const address = await client.selfAddress();
   * ```
   */
  selfAddress: () => Promise<string | undefined>;
  /**
   * Set the mime-types that should be used when using the {@link Client.send} method.
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   * clientId: 'my-client',
   * nymApiUrl: 'https://validator.nymtech.net/api',
   * });
   * await client.setTextMimeTypes(['text/plain', 'application/json']);
   * ```
   * @param mimeTypes
   * @see {@link MimeTypes}
   * @see {@link Client.send}
   * @see {@link Client.getTextMimeTypes}
   */
  setTextMimeTypes: (mimeTypes: string[]) => void;
  /**
   * Get the mime-types that are automatically converted to strings.
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   * clientId: 'my-client',
   * nymApiUrl: 'https://validator.nymtech.net/api',
   * });
   * const mimeTypes = await client.getTextMimeTypes();
   * ```
   * @see {@link MimeTypes}
   * @see {@link Payload}
   * @see {@link Client.send}
   * @see {@link Client.setTextMimeTypes}
   */
  getTextMimeTypes: () => Promise<string[]>;
  /**
   * Send some data through the mixnet message.
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   *  clientId: 'my-client',
   *  nymApiUrl: 'https://validator.nymtech.net/api',
   * });
   * await client.send({
   *  payload: 'Hello world',
   *  recipient: // recipient address,
   * });
   * ```
   * @see {@link MimeTypes}
   * @see {@link Payload}
   */
  send: (args: { payload: Payload; recipient: string; replySurbs?: number }) => Promise<void>;
  /**
   * Send a raw payload, without any mime-type conversion.
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   *  clientId: 'my-client',
   *  nymApiUrl: 'https://validator.nymtech.net/api',
   * });
   * const payload = new Uint8Array([1, 2, 3]);
   * await client.rawSend({
   *  payload,
   *  recipient: // recipient address,
   * });
   * ```
   * @see {@link MimeTypes}
   * @see {@link Payload}
   */
  rawSend: (args: { payload: Uint8Array; recipient: string; replySurbs?: number }) => Promise<void>;
}

/**
 * The configuration passed to the {@link Client.start} method of the {@link Client}
 */
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

export interface Events {
  /**
   * @see {@link LoadedEvent}
   * @example
   * ```typescript
   * events.subscribeToLoaded((e) => {
   *  console.log(e.args); // { loaded: true }
   * });
   * ```
   */
  subscribeToLoaded: EventHandlerSubscribeFn<LoadedEvent>;
  /**
   * @see {@link ConnectedEvent}
   * @example
   * ```typescript
   * events.subscribeConnected((e) => {
   *  console.log(e.args.address); // Client address
   * });
   *
   */
  subscribeToConnected: EventHandlerSubscribeFn<ConnectedEvent>;
  /**
   * @returns {@link EventHandlerUnsubscribeFn}
   * @see {@link StringMessageReceivedEvent}
   * @example
   * ```typescript
   * const unsubscribe = events.subscribeToTextMessageReceivedEvent((e) => {
   *  console.log(e.args.payload); // string
   * });
   *
   * // Stop listening to the event
   * unsubscribe();
   * ```
   */
  subscribeToTextMessageReceivedEvent: EventHandlerSubscribeFn<StringMessageReceivedEvent>;
  /**
   * @returns {@link EventHandlerUnsubscribeFn}
   * @see {@link BinaryMessageReceivedEvent}
   * @example
   * ```typescript
   * const unsubscribe = events.subscribeToBinaryMessageReceivedEvent((e) => {
   *  console.log(e.args.payload); // Uint8Array
   * });
   *
   * // Stop listening to the event
   * unsubscribe();
   * ```
   */
  subscribeToBinaryMessageReceivedEvent: EventHandlerSubscribeFn<BinaryMessageReceivedEvent>;
  /**
   * @returns {@link EventHandlerUnsubscribeFn}
   * @see {@link RawMessageReceivedEvent}
   * @example
   * ```typescript
   * const unsubscribe = events.subscribeToRawMessageReceivedEvent((e) => {
   *  console.log(e.args.payload); // Uint8Array
   * });
   *
   * // Stop listening to the event
   * unsubscribe();
   * ```
   */
  subscribeToRawMessageReceivedEvent: EventHandlerSubscribeFn<RawMessageReceivedEvent>;
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
 * The **EventHandlerSubscribeFn** is a function that takes a callback of type {@link EventHandlerFn}
 *
 * @see {@link Events}
 * @see {@link EventHandlerFn}
 * @see {@link EventHandlerUnsubscribeFn}
 */
export type EventHandlerSubscribeFn<E> = (fn: EventHandlerFn<E>) => EventHandlerUnsubscribeFn;

/**
 * The **EventHandlerFn** is a callback function that is passed to the {@link EventHandlerSubscribeFn}
 * @see {@link Events}
 * @see {@link EventHandlerFn}
 * @see {@link EventHandlerSubscribeFn}
 */
export type EventHandlerFn<E> = (e: E) => void | Promise<void>;

/**
 * The **EventHandlerUnsubscribeFn** function is returned by the {@link EventHandlerSubscribeFn}
 * and can be used to stop listening for particular events
 * @see {@link Events}
 * @see {@link EventHandlerFn}
 * @see {@link EventHandlerSubscribeFn}
 */
export type EventHandlerUnsubscribeFn = () => void;
