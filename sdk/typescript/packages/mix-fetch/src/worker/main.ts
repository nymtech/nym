/* eslint-disable no-restricted-globals */
import { setupMixFetch, disconnectMixFetch } from '@nymproject/mix-fetch-wasm';
import * as Comlink from 'comlink';
import { runIfDebugEnabled } from '../debug';
import type { IMixFetchWebWorker, LoadedEvent } from '../types';
import { EventKinds, ResponseBodyConfigMap, ResponseBodyConfigMapDefaults } from '../types';
import { handleResponseMimeTypes } from './handle-response-mime-types';

/**
 * Helper method to send typed messages.
 * @param event   The strongly typed message to send back to the calling thread.
 */
// eslint-disable-next-line no-restricted-globals
const postMessageWithType = <E>(event: E) => self.postMessage(event);
export async function run() {
  const { mixFetch } = self as any;
  let responseBodyConfigMap: ResponseBodyConfigMap = ResponseBodyConfigMapDefaults;

  const mixFetchWebWorker: IMixFetchWebWorker = {
    mixFetch: async (url, args) => {
      runIfDebugEnabled(() => {
        console.log('[Worker] --- mixFetch ---', { url, args });
      });

      const response: Response = await mixFetch(url, args);

      runIfDebugEnabled(() => {
        console.log('[Worker]', { response, json: JSON.stringify(response, null, 2) });
      });

      const bodyResponse = await handleResponseMimeTypes(response, responseBodyConfigMap);
      runIfDebugEnabled(() => {
        console.log('[Worker]', { bodyResponse });
      });

      const headers: any = {};
      response.headers.forEach((value, key) => {
        headers[key] = value;
      });

      const output = {
        body: bodyResponse,
        url: response.url,
        headers,
        status: response.status,
        statusText: response.statusText,
        type: response.type,
        ok: response.ok,
        redirected: response.redirected,
      };

      runIfDebugEnabled(() => {
        console.log('[Worker]', { output });
      });

      return output;
    },
    setupMixFetch: async (opts) => {
      runIfDebugEnabled(() => {
        console.log('[Worker] --- setupMixFetch ---', { opts });
      });
      if (opts?.responseBodyConfigMap) {
        responseBodyConfigMap = opts.responseBodyConfigMap;
      }
      await setupMixFetch(opts || {});
    },
    disconnectMixFetch: async () => {
      runIfDebugEnabled(() => {
        console.log('[Worker] --- disconnectMixFetch ---');
      });
      await disconnectMixFetch();
    },
  };

  // start comlink listening for messages and handle them above
  Comlink.expose(mixFetchWebWorker);

  // notify any listeners that the web worker has loaded - HOWEVER, mixFetch hasn't been setup and the client started
  // call `setupMixFetch` from the main thread to start the Nym client
  postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
}
