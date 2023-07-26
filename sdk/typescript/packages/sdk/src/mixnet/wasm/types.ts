import type { DebugWasm } from './types-from-wasm-pack';

export * from './types-from-wasm-pack';

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
  /**
   * Stop the client.
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   *  clientId: 'my-client',
   *  nymApiUrl: 'https://validator.nymtech.net',
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
   *  nymApiUrl: 'https://validator.nymtech.net',
   * });
   * const address = await client.selfAddress();
   * ```
   */
  selfAddress: () => Promise<string | undefined>;

  setTextMimeTypes: (mimeTypes: string[]) => void;
  getTextMimeTypes: () => Promise<string[]>;
  /**
   * Send a string message.
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   *  clientId: 'my-client',
   *  nymApiUrl: 'https://validator.nymtech.net',
   * });
   * await client.send({
   *  payload: 'Hello world',
   *  recipient: // recipient address,
   * });
   * ```
   */
  send: (args: { payload: Payload; recipient: string; replySurbs?: number }) => Promise<void>;
  /**
   * Send a raw payload, without any mime-type conversion.
   * @example
   * ```typescript
   * const client = await createNymMixnetClient();
   * await client.start({
   *  clientId: 'my-client',
   *  nymApiUrl: 'https://validator.nymtech.net',
   * });
   * const payload = new Uint8Array([1, 2, 3]);
   * await client.rawSend({
   *  payload,
   *  recipient: // recipient address,
   * });
   * ```
   */
  rawSend: (args: { payload: Uint8Array; recipient: string; replySurbs?: number }) => Promise<void>;
}

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

export type EventHandlerFn<E> = (e: E) => void | Promise<void>;

export type EventHandlerSubscribeFn<E> = (fn: EventHandlerFn<E>) => EventHandlerUnsubscribeFn;

export type EventHandlerUnsubscribeFn = () => void;
