/* eslint-disable no-console,no-restricted-globals */
/// <reference path="../../../../nym-client-wasm/nym_client_wasm.d.ts" />
/**
 * NB: URL syntax is used so that bundlers like webpack can load this package's code when inside the final bundle
 * the files from ../../../../nym-client-wasm will be copied into the dist directory of this package, so all import
 * paths are _relative to the output directory_ of this package (`dist`) - don't get confused!
 */
import * as Comlink from 'comlink';
import type {
  ConnectedEvent,
  IWebWorker,
  LoadedEvent,
  OnStringMessageFn,
  OnBinaryMessageFn,
  OnConnectFn,
  StringMessageReceivedEvent,
  BinaryMessageReceivedEvent,
  NymClientConfig,
} from './types';
import { EventKinds } from './types';

// web workers are only allowed to load external scripts as the load
importScripts(new URL('./nym_client_wasm.js', import.meta.url));

console.log('[Nym WASM client] Starting Nym WASM web worker...');

// again, construct a URL that can be used by a bundler to repackage the WASM binary
const wasmUrl = new URL('./nym_client_wasm_bg.wasm', import.meta.url);

/**
 * Helper method to send typed messages.
 * @param event   The strongly typed message to send back to the calling thread.
 */
const postMessageWithType = <E>(event: E) => self.postMessage(event);

// ------------ the 1st byte of messages is the kind from the list below ------------
const PAYLOAD_KIND_TEXT = 0;
const PAYLOAD_KIND_BINARY = 1;

/**
 * This class holds the state of the Nym WASM client and provides any interop needed.
 */
class ClientWrapper {
  client: wasm_bindgen.NymClient | null = null;
  clientBuilder: wasm_bindgen.NymClientBuilder | null = null;

  /**
   * Creates the WASM client and initialises it.
   */
  init = async (
    config: wasm_bindgen.Config,
    onMessageHandler: OnBinaryMessageFn,
  ) => {

    this.clientBuilder = new wasm_bindgen.NymClientBuilder(config, onMessageHandler);

    // NB: because we set the `kind` byte in the message payload first, we don't need to bother to try to parse
    // all messages as string
    // if (onStringMessageHandler) {
    //   this.client.set_on_message(onStringMessageHandler);
    // }
  };

  /**
   * Returns the address of this client.
   */
  selfAddress = () => {
    if (!this.client) {
      console.error('Client has not been initialised. Please call `init` first.');
      return undefined;
    }

    return this.client.self_address();
  };

  /**
   * Connects to the gateway and starts the client sending traffic.
   */
  start = async () => {
    if (!this.clientBuilder) {
      console.error('Client builder has not been initialised. Please call `init` first.');
      return;
    }

    // this is current limitation of wasm in rust - for async methods you can't take self by reference...
    // I'm trying to figure out if I can somehow hack my way around it, but for time being you have to re-assign
    // the object (it's the same one)
    this.client = await this.clientBuilder.start_client();
  };

  sendMessage = async ({ payload, recipient }: { recipient: string; payload: string }) => {
    if (!this.client) {
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }
    const message = wasm_bindgen.create_binary_message_from_string(PAYLOAD_KIND_TEXT, payload);
    this.client = await this.client.send_regular_message(message, recipient);
  };

  sendBinaryMessage = async ({
    payload,
    recipient,
    headers,
  }: {
    recipient: string;
    payload: Uint8Array;
    headers?: string;
  }) => {
    if (!this.client) {
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }
    const message = wasm_bindgen.create_binary_message_with_headers(PAYLOAD_KIND_BINARY, payload, headers || '');
    this.client = await this.client.send_regular_message(message, recipient);
  };
}

// load WASM binary
wasm_bindgen(wasmUrl)
  .then((importResult) => {
    // sets up better stack traces in case of in-rust panics
    importResult.set_panic_hook();

    // this wrapper handles any state that the wasm-pack interop needs, e.g. holding an instance of the instantiated WASM code
    const wrapper = new ClientWrapper();

    const startHandler = async (config: NymClientConfig) => {
      // fetch the gateway details (randomly chosen if no preferred gateway is set)
      const gatewayEndpoint = await wasm_bindgen.get_gateway(
        config.nymApiUrl,
        config.preferredGatewayIdentityKey,
      );

      // set a different gatewayListener in order to avoid workaround ws over https error
      if (config.gatewayListener)
        gatewayEndpoint.gateway_listener = config.gatewayListener;

      // create the client, passing handlers for events
      wrapper.init(
        new wasm_bindgen.Config(
          config.clientId,
          config.nymApiUrl,
          gatewayEndpoint,
          config.debug || wasm_bindgen.default_debug(),
        ),
        async (message: Uint8Array) => {
          try {
            const { kind, payload, headers } = await wasm_bindgen.parse_binary_message_with_headers(message);
            switch (kind) {
              case PAYLOAD_KIND_TEXT: {
                const stringMessage = await wasm_bindgen.parse_string_message_with_headers(message);
                postMessageWithType<StringMessageReceivedEvent>({
                  kind: EventKinds.StringMessageReceived,
                  args: { kind, payload: stringMessage.payload },
                });
                break;
              }
              case PAYLOAD_KIND_BINARY:
                postMessageWithType<BinaryMessageReceivedEvent>({
                  kind: EventKinds.BinaryMessageReceived,
                  args: { kind, payload, headers: headers || '' },
                });
                break;
              default:
                console.error('Could not determine message kind from 1st byte of message', { message, kind, payload });
            }
          } catch (e) {
            console.error('Failed to parse binary message', e);
          }
        },
      );

      // start the client sending traffic
      await wrapper.start();

      // get the address
      const address = wrapper.selfAddress();
      postMessageWithType<ConnectedEvent>({ kind: EventKinds.Connected, args: { address } });
    };

    // implement the public logic of this web worker (message exchange between the worker and caller is done by https://www.npmjs.com/package/comlink)
    const webWorker: IWebWorker = {
      start(config) {
        console.log('[Nym WASM client] Starting...', { config });
        startHandler(config).catch((e) => console.error('[Nym WASM client] Failed to start', e));
      },
      selfAddress() {
        return wrapper.selfAddress();
      },
      sendMessage(args) {
        wrapper.sendMessage(args).catch((e) => console.error('[Nym WASM client] Failed to send message', e));
      },
      sendBinaryMessage(args) {
        wrapper.sendBinaryMessage(args).catch((e) => console.error('[Nym WASM client] Failed to send message', e));
      },
    };

    // start comlink listening for messages and handle them above
    Comlink.expose(webWorker);

    // notify any listeners that the web worker has loaded - HOWEVER, the client has not been created and connected,
    // listen for EventKinds.Connected before sending messages
    postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
  })
  .catch((e) => {
    console.error('[Worker thread] failed to start', e);
  });
