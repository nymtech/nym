/* eslint-disable no-console,no-restricted-globals */
/// <reference path="../../../../nym-client-wasm/nym_client_wasm.d.ts" />
/**
 * NB: URL syntax is used so that bundlers like webpack can load this package's code when inside the final bundle
 * the files from ../../../../nym-client-wasm will be copied into the dist directory of this package, so all import
 * paths are _relative to the output directory_ of this package (`dist`) - don't get confused!
 */
import * as Comlink from 'comlink';
import type {
  BinaryMessageReceivedEvent,
  ConnectedEvent,
  IWebWorker,
  LoadedEvent,
  NymClientConfig,
  OnRawPayloadFn,
  StringMessageReceivedEvent,
} from './types';
import { EventKinds, MimeTypes } from './types';

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
  init = (config: Config, onRawPayloadHandler?: OnRawPayloadFn) => {
    const onMessageHandler = (message: Uint8Array) => {
      try {
        if (onRawPayloadHandler) {
          onRawPayloadHandler(message);
        }
      } catch (e) {
        console.error('Unhandled exception in `ClientWrapper.onRawPayloadHandler`: ', e);
      }
    };

    this.builder = new NymClientBuilder(config, onMessageHandler);
  };

  /**
   * Sets the mime-types that will be parsed for UTF-8 string content.
   *
   * @param mimeTypes An array of mime-types to treat as having string content.
   */
  setTextMimeTypes = (mimeTypes: string[]) => {
    this.mimeTypes = mimeTypes;
  };

  /**
   * Gest the mime-types that are considered as string and will be automatically converted to byte arrays.
   */
  getTextMimeTypes = () => this.mimeTypes;

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
    if (!this.builder) {
      console.error('Client config has not been initialised. Please call `init` first.');
      return;
    }

    // this is current limitation of wasm in rust - for async methods you can't take self by reference...
    // I'm trying to figure out if I can somehow hack my way around it, but for time being you have to re-assign
    // the object (it's the same one)
    this.client = await this.builder.start_client();
  };

  /**
   * Stops the client and cleans up.
   */
  stop = () => {
    if (!this.client) {
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }
    this.client.free();
    this.client = null;
  };

  send = async ({
    payload,
    recipient,
    replySurbs = 0,
  }: {
    recipient: string;
    payload: Uint8Array;
    replySurbs?: number;
  }) => {
    if (!this.client) {
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }
    // TODO: currently we don't do anything with the result, it needs some typing and exposed back on the main thread
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const res = await this.client.send_anonymous_message(payload, recipient, replySurbs);
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
        () => {
          console.log();
        },
        undefined,
        async (message) => {
          try {
            const decodedPayload = decode_payload(message);
            const { payload, headers } = decodedPayload;
            const mimeType = decodedPayload.mimeType as MimeTypes;

            if (wrapper.getTextMimeTypes().includes(mimeType)) {
              const stringMessage = parse_utf8_string(payload);

              postMessageWithType<StringMessageReceivedEvent>({
                kind: EventKinds.StringMessageReceived,
                args: { mimeType, payload: stringMessage, payloadRaw: payload, headers },
              });
              return;
            }

            postMessageWithType<BinaryMessageReceivedEvent>({
              kind: EventKinds.BinaryMessageReceived,
              args: { mimeType, payload, headers },
            });
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
      stop() {
        wrapper.stop();
      },
      selfAddress() {
        return wrapper.selfAddress();
      },
      setTextMimeTypes(mimeTypes) {
        wrapper.setTextMimeTypes(mimeTypes);
      },
      getTextMimeTypes() {
        return wrapper.getTextMimeTypes();
      },
      send(args) {
        const {
          recipient,
          replySurbs,
          payload: { mimeType, headers },
        } = args;
        let payloadBytes = new Uint8Array();
        if (mimeType && wrapper.getTextMimeTypes().includes(mimeType) && typeof args.payload.message === 'string') {
          payloadBytes = utf8_string_to_byte_array(args.payload.message);
        } else if (typeof args.payload.message !== 'string') {
          payloadBytes = args.payload.message;
        } else {
          console.error(
            '[Nym WASM client] Payload is a string. It should be a UintArray, or the mime-type should be set with `setTextMimeTypes` for auto-conversion',
          );
          return;
        }
        const payload = encode_payload_with_headers(
          mimeType || MimeTypes.ApplicationOctetStream,
          payloadBytes,
          headers,
        );
        wrapper
          .send({ payload, recipient, replySurbs })
          .catch((e) => console.error('[Nym WASM client] Failed to send message', e));
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
