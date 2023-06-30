import type { DebugWasm } from './types-from-wasm-pack';

export * from './types-from-wasm-pack';

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
  debug?: DebugWasm;
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
