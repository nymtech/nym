/// <reference path="../../../../nym-client-wasm/nym_client_wasm.d.ts" />

export type OnMessageFn = (message: string) => void;

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
   * Optional. Settings for the WASM client.
   */
  debug?: wasm_bindgen.Debug;
}

export interface IWebWorker {
  start: (config: NymClientConfig) => void;
  selfAddress: () => string | undefined;
  sendMessage: (args: { message: string; recipient: string }) => void;
  sendBinaryMessage: (args: { message: Uint8Array; recipient: string }) => void;
}

export enum EventKinds {
  Loaded = 'Loaded',
  Connected = 'Connected',
  TextMessageReceived = 'TextMessageReceived',
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

export interface TextMessageReceivedEvent {
  kind: EventKinds.TextMessageReceived;
  args: {
    message: string;
  };
}

export interface BinaryMessageReceivedEvent {
  kind: EventKinds.TextMessageReceived;
  args: {
    message: Uint8Array;
  };
}

export interface IWebWorkerEvents {
  subscribeToLoaded: EventHandlerSubscribeFn<LoadedEvent>;
  subscribeToConnected: EventHandlerSubscribeFn<ConnectedEvent>;
  subscribeToTextMessageReceivedEvent: EventHandlerSubscribeFn<TextMessageReceivedEvent>;
  subscribeToBinaryMessageReceivedEvent: EventHandlerSubscribeFn<BinaryMessageReceivedEvent>;
}
