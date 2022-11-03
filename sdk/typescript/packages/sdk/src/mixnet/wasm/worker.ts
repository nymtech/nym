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
  OnMessageFn,
  OnConnectFn,
  TextMessageReceivedEvent,
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

/**
 * This class holds the state of the Nym WASM client and provides any interop needed.
 */
class ClientWrapper {
  client: wasm_bindgen.NymClient | null = null;

  /**
   * Creates the WASM client and initialises it.
   */
  init = (config: wasm_bindgen.Config, onConnectHandler: OnConnectFn, onMessageHandler: OnMessageFn) => {
    this.client = new wasm_bindgen.NymClient(config);
    this.client.set_on_message(onMessageHandler);
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
    if (!this.client) {
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }

    // this is current limitation of wasm in rust - for async methods you can't take self by reference...
    // I'm trying to figure out if I can somehow hack my way around it, but for time being you have to re-assign
    // the object (it's the same one)
    this.client = await this.client.start();
  };

  sendMessage = async ({ message, recipient }: { recipient: string; message: string }) => {
    if (!this.client) {
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }

    this.client = await this.client.send_message(message, recipient);
  };

  sendBinaryMessage = async ({ message, recipient }: { recipient: string; message: Uint8Array }) => {
    if (!this.client) {
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }

    this.client = await this.client.send_binary_message(message, recipient);
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
        config.validatorApiUrl,
        config.preferredGatewayIdentityKey,
      );

      // create the client, passing handlers for events
      wrapper.init(
        new wasm_bindgen.Config(
          config.clientId,
          config.validatorApiUrl,
          gatewayEndpoint,
          config.debug || wasm_bindgen.default_debug(),
        ),
        () => {
          console.log();
        },
        (message) => {
          postMessageWithType<TextMessageReceivedEvent>({ kind: EventKinds.TextMessageReceived, args: { message } });
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
