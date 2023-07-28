import * as Comlink from 'comlink';
import InlineWasmWebWorker from 'web-worker:./worker';
import {
  BinaryMessageReceivedEvent,
  ConnectedEvent,
  EventKinds,
  IWebWorker,
  Client,
  Events,
  LoadedEvent,
  MimeTypes,
  RawMessageReceivedEvent,
  StringMessageReceivedEvent,
} from './types';
import { createSubscriptions } from './subscriptions';

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
 *
 */
export interface NymMixnetClient {
  client: Client;
  events: Events;
}

/**
 * Create a client to send and receive traffic from the Nym mixnet.
 * @required
 * @returns
 * @example
 * ```typescript
 * const client = await createNymMixnetClient();
 * ```
 */
export const createNymMixnetClient = async (options?: NymMixnetClientOptions): Promise<NymMixnetClient> => {
  // create a web worker that runs the WASM client on another thread and wait until it signals that it is ready
  // eslint-disable-next-line @typescript-eslint/no-use-before-define
  const worker = await createWorker();

  const subscriptions = createSubscriptions();
  const { getSubscriptions, addSubscription } = subscriptions;

  // listen to messages from the worker, parse them and let the subscribers handle them, catching any unhandled exceptions
  worker.addEventListener('message', (msg) => {
    if (msg.data && msg.data.kind) {
      const subscribers = getSubscriptions(msg.data.kind);
      (subscribers || []).forEach((s) => {
        try {
          // let the subscriber handle the message
          s(msg.data);
        } catch (e) {
          // eslint-disable-next-line no-console
          console.error('Unhandled error in event handler', msg.data, e);
        }
      });
    }
  });

  // manage the subscribers, returning self-unsubscribe methods
  const events: Events = {
    subscribeToConnected: (handler) => addSubscription<ConnectedEvent>(EventKinds.Connected, handler),
    subscribeToLoaded: (handler) => addSubscription<LoadedEvent>(EventKinds.Loaded, handler),
    subscribeToTextMessageReceivedEvent: (handler) =>
      addSubscription<StringMessageReceivedEvent>(EventKinds.StringMessageReceived, handler),
    subscribeToBinaryMessageReceivedEvent: (handler) =>
      addSubscription<BinaryMessageReceivedEvent>(EventKinds.BinaryMessageReceived, handler),
    subscribeToRawMessageReceivedEvent: (handler) =>
      addSubscription<RawMessageReceivedEvent>(EventKinds.RawMessageReceived, handler),
  };

  // let comlink handle interop with the web worker
  const client: Client = Comlink.wrap<IWebWorker>(worker);

  // set any options
  if (options?.autoConvertStringMimeTypes) {
    await client.setTextMimeTypes(options.autoConvertStringMimeTypes);
  } else {
    // set some sensible defaults for text mime types
    await client.setTextMimeTypes([MimeTypes.ApplicationJson, MimeTypes.TextPlain]);
  }

  // pass the client interop and subscription manage back to the caller
  return {
    client,
    events,
  };
};

/**
 * Async method to create a web worker that runs the Nym client on another thread. It will only return once the worker
 * has passed back a `Loaded` event to the calling thread.
 *
 * @return The instance of the web worker.
 */
const createWorker = async () =>
  new Promise<Worker>((resolve, reject) => {
    // rollup will inline the built worker script, so that when the SDK is used in
    // other projects, they will not need to mess around trying to bundle it
    // however, it will make this SDK bundle bigger because of Base64 inline data
    const worker = new InlineWasmWebWorker();

    worker.addEventListener('error', reject);
    worker.addEventListener(
      'message',
      (msg) => {
        worker.removeEventListener('error', reject);
        if (msg.data?.kind === EventKinds.Loaded) {
          resolve(worker);
        } else {
          reject(msg);
        }
      },
      { once: true },
    );
  });
