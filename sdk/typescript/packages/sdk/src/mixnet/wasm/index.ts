import * as Comlink from 'comlink';
import InlineWasmWebWorker from 'web-worker:./worker';
import {
  BinaryMessageReceivedEvent,
  ConnectedEvent,
  EventHandlerFn,
  EventKinds,
  IWebWorker,
  IWebWorkerAsync,
  IWebWorkerEvents,
  LoadedEvent,
  MimeTypes,
  StringMessageReceivedEvent,
  RawMessageReceivedEvent,
} from './types';

/**
 * Client for the Nym mixnet.
 */
export interface NymMixnetClient {
  client: IWebWorkerAsync;
  events: IWebWorkerEvents;
}

/**
 * Create a client to send and receive traffic from the Nym mixnet.
 *
 */
export const createNymMixnetClient = async (options?: {
  autoConvertStringMimeTypes?: string[] | MimeTypes[];
}): Promise<NymMixnetClient> => {
  // create a web worker that runs the WASM client on another thread and wait until it signals that it is ready
  // eslint-disable-next-line @typescript-eslint/no-use-before-define
  const worker = await createWorker();

  // stores the subscriptions for events
  const subscriptions: {
    [key: string]: Array<EventHandlerFn<unknown>>;
  } = {};

  /**
   * Helper method to get typed subscriptions
   */
  const getSubscriptions = <E>(key: EventKinds): Array<EventHandlerFn<E>> => {
    if (!subscriptions[key]) {
      subscriptions[key] = [];
    }
    return subscriptions[key] as Array<EventHandlerFn<E>>;
  };

  // listen to messages from the worker, parse them and let the subscribers handle them, catching any unhandled exceptions
  worker.addEventListener('message', (msg) => {
    if (msg.data && msg.data.kind) {
      const subscribers = subscriptions[msg.data.kind];
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
  const events: IWebWorkerEvents = {
    subscribeToConnected: (handler) => {
      getSubscriptions<ConnectedEvent>(EventKinds.Connected).push(handler);
      return () => {
        getSubscriptions<ConnectedEvent>(EventKinds.Connected).unshift(handler);
      };
    },
    subscribeToLoaded: (handler) => {
      getSubscriptions<LoadedEvent>(EventKinds.Loaded).push(handler);
      return () => {
        getSubscriptions<LoadedEvent>(EventKinds.Loaded).unshift(handler);
      };
    },
    subscribeToTextMessageReceivedEvent: (handler) => {
      getSubscriptions<StringMessageReceivedEvent>(EventKinds.StringMessageReceived).push(handler);
      return () => {
        getSubscriptions<StringMessageReceivedEvent>(EventKinds.StringMessageReceived).unshift(handler);
      };
    },
    subscribeToBinaryMessageReceivedEvent: (handler) => {
      getSubscriptions<BinaryMessageReceivedEvent>(EventKinds.BinaryMessageReceived).push(handler);
      return () => {
        getSubscriptions<BinaryMessageReceivedEvent>(EventKinds.BinaryMessageReceived).unshift(handler);
      };
    },
    subscribeToRawMessageReceivedEvent: (handler) => {
      getSubscriptions<RawMessageReceivedEvent>(EventKinds.RawMessageReceived).push(handler);
      return () => {
        getSubscriptions<RawMessageReceivedEvent>(EventKinds.RawMessageReceived).unshift(handler);
      };
    },
  };

  // let comlink handle interop with the web worker
  const client: IWebWorkerAsync = Comlink.wrap<IWebWorker>(worker);

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
