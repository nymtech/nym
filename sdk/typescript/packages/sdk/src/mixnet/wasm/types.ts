/// <reference path="../../../../nym-client-wasm/nym_client_wasm.d.ts" />

export type OnStringMessageFn = (message: string) => void;

export type OnBinaryMessageFn = (message: Uint8Array) => void;

export type OnConnectFn = (address?: string) => void;

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
  validatorApiUrl: string;

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
  debug?: wasm_bindgen.Debug;
}

export interface Headers {
  senderAddress?: string;
  [key: string]: unknown;
}

export interface IWebWorker {
  start: (config: NymClientConfig) => void;
  selfAddress: () => string | undefined;
  sendMessage: (args: { payload: string; recipient: string; headers?: Headers }) => void;
  sendBinaryMessage: (args: { payload: Uint8Array; recipient: string; headers?: Headers }) => void;
}

export enum EventKinds {
  Loaded = 'Loaded',
  Connected = 'Connected',
  StringMessageReceived = 'StringMessageReceived',
  BinaryMessageReceived = 'BinaryMessageReceived',
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
    kind: number;
    payload: string;
    headers?: Headers;
  };
}

export interface BinaryMessageReceivedEvent {
  kind: EventKinds.BinaryMessageReceived;
  args: {
    kind: number;
    payload: Uint8Array;
    headers?: Headers;
  };
}

export interface IWebWorkerEvents {
  subscribeToLoaded: EventHandlerSubscribeFn<LoadedEvent>;
  subscribeToConnected: EventHandlerSubscribeFn<ConnectedEvent>;
  subscribeToTextMessageReceivedEvent: EventHandlerSubscribeFn<StringMessageReceivedEvent>;
  subscribeToBinaryMessageReceivedEvent: EventHandlerSubscribeFn<BinaryMessageReceivedEvent>;
}
