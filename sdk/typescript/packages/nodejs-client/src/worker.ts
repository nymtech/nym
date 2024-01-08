import './polyfill';

import * as Comlink from 'comlink';
import { parentPort } from 'worker_threads';
import '@nymproject/nym-client-wasm-node/nym_client_wasm_bg.wasm';

import {
  NymClient,
  decode_payload,
  encode_payload_with_headers,
  parse_utf8_string,
  utf8_string_to_byte_array,
  ClientOpts,
} from '@nymproject/nym-client-wasm-node';

import type {
  BinaryMessageReceivedEvent,
  ConnectedEvent,
  IWebWorker,
  LoadedEvent,
  OnRawPayloadFn,
  RawMessageReceivedEvent,
  StringMessageReceivedEvent,
} from './types';

import nodeEndpoint from './node-adapter';
import { EventKinds, MimeTypes } from './types';

// eslint-disable-next-line no-console
console.log('[Nym WASM client] Starting Nym WASM web worker...');

/**
 * Helper method to send typed messages.
 * @param event   The strongly typed message to send back to the calling thread.
 * see https://nodejs.org/api/worker_threads.html#workerparentport
 */
const postMessageWithType = <E>(event: E) => parentPort?.postMessage(event);

/**
 * This class holds the state of the Nym WASM client and provides any interop needed.
 */
class ClientWrapper {
  client: NymClient | null = null;

  mimeTypes: string[] = [MimeTypes.TextPlain, MimeTypes.ApplicationJson];

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
      // eslint-disable-next-line no-console
      console.error('Client has not been initialised. Please call `init` first.');
      return undefined;
    }
    return this.client.self_address();
  };

  /**
   * Connects to the gateway and starts the client sending traffic.
   */
  start = async (opts?: ClientOpts, onRawPayloadHandler?: OnRawPayloadFn) => {
    const onMessageHandler = (message: Uint8Array) => {
      try {
        if (onRawPayloadHandler) {
          onRawPayloadHandler(message);
        }
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error('Unhandled exception in `ClientWrapper.onRawPayloadHandler`: ', e);
      }
    };

    // this is current limitation of wasm in rust - for async methods you can't take self by reference...
    // I'm trying to figure out if I can somehow hack my way around it, but for time being you have to re-assign
    // the object (it's the same one)
    this.client = await new NymClient(onMessageHandler, opts);
  };

  /**
   * Stops the client and cleans up.
   */
  stop = () => {
    if (!this.client) {
      // eslint-disable-next-line no-console
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }
    this.client.free();
    this.client = null;
  };

  send = async ({
    payload,
    recipient,
    replySurbs = 10,
  }: {
    payload: Uint8Array;
    recipient: string;
    replySurbs?: number;
  }) => {
    if (!this.client) {
      // eslint-disable-next-line no-console
      console.error('Client has not been initialised. Please call `init` first.');
      return;
    }
    // TODO: currently we don't do anything with the result, it needs some typing and exposed back on the main thread
    await this.client.send_anonymous_message(payload, recipient, replySurbs);
  };
}

// this wrapper handles any state that the wasm-pack interop needs, e.g. holding an instance of the instantiated WASM code
const wrapper = new ClientWrapper();

const startHandler = async (opts?: ClientOpts) => {
  // create the client, passing handlers for events
  await wrapper.start(opts, async (message) => {
    // fire an event with the raw message
    postMessageWithType<RawMessageReceivedEvent>({
      kind: EventKinds.RawMessageReceived,
      args: { payload: message },
    });
    try {
      // try to decode the payload to extract the mime-type, headers and payload body
      const decodedPayload = decode_payload(message);
      const { payload, headers } = decodedPayload;
      const mimeType = decodedPayload.mimeType as MimeTypes;
      if (wrapper.getTextMimeTypes().includes(mimeType)) {
        const stringMessage = parse_utf8_string(payload);
        // the payload is a string type (in the options at creation time, string mime-types are set, or fall back
        // to defaults, such as `text/plain`, `application/json`, etc)
        postMessageWithType<StringMessageReceivedEvent>({
          kind: EventKinds.StringMessageReceived,
          args: { mimeType, payload: stringMessage, payloadRaw: payload, headers },
        });
        return;
      }
      // the payload is a binary type
      postMessageWithType<BinaryMessageReceivedEvent>({
        kind: EventKinds.BinaryMessageReceived,
        args: { mimeType, payload, headers },
      });
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error('Failed to parse binary message', e);
    }
  });

  // get the address
  const address = wrapper.selfAddress();
  postMessageWithType<ConnectedEvent>({ kind: EventKinds.Connected, args: { address } });
};

// implement the public logic of this web worker (message exchange between the worker and caller is done by https://www.npmjs.com/package/comlink)
const webWorker: IWebWorker = {
  start(opts?: ClientOpts) {
    // eslint-disable-next-line no-console
    startHandler(opts).catch((e) => console.error('[Nym WASM client] Failed to start', e));
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
      // eslint-disable-next-line no-console
      console.error(
        "[Nym WASM client] Payload is a string. It should be a UintArray, or the mime-type should be set with `setTextMimeTypes` or in the options for `init({ autoConvertStringMimeTypes: ['text/plain', 'application/json'] })` for auto-conversion",
      );
      return;
    }
    const payload = encode_payload_with_headers(mimeType || MimeTypes.ApplicationOctetStream, payloadBytes, headers);
    wrapper
      .send({ payload, recipient, replySurbs })
      // eslint-disable-next-line no-console
      .catch((e) => console.error('[Nym WASM client] Failed to send message', e));
  },
  rawSend(args) {
    const { recipient, payload, replySurbs } = args;
    wrapper
      .send({ payload, replySurbs, recipient })
      // eslint-disable-next-line no-console
      .catch((e) => console.error('[Nym WASM client] Failed to send message', e));
  },
};

// start comlink listening for messages and handle them above, if we are on a worker thread.
if (parentPort) {
  Comlink.expose(webWorker, nodeEndpoint(parentPort));
}

// notify any listeners that the web worker has loaded - HOWEVER, the client has not been created and connected,
// listen for EventKinds.Connected before sending messages
postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
