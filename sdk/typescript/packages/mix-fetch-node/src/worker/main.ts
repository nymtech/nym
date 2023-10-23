/* eslint-disable no-console */
import type { IMixFetchWebWorker, LoadedEvent } from '../types';

import * as Comlink from 'comlink';
import { parentPort } from 'node:worker_threads';
import { setupMixFetch, disconnectMixFetch } from '@nymproject/mix-fetch-wasm-node';

import nodeEndpoint from '../node-adapter';
import { EventKinds, ResponseBodyConfigMap, ResponseBodyConfigMapDefaults } from '../types';
import { handleResponseMimeTypes } from './handle-response-mime-types';

/**
 * Helper method to send typed messages.
 * @param event   The strongly typed message to send back to the calling thread.
 */
const postMessageWithType = <E>(event: E) => parentPort?.postMessage(event);

export async function run() {
  const { mixFetch } = globalThis as any;
  let responseBodyConfigMap: ResponseBodyConfigMap = ResponseBodyConfigMapDefaults;

  const mixFetchWebWorker: IMixFetchWebWorker = {
    mixFetch: async (url, args) => {
      console.log('[Worker] --- mixFetch ---', { url, args });

      const response: Response = await mixFetch(url, args);
      console.log('[Worker]', { response, json: JSON.stringify(response, null, 2) });

      const bodyResponse = await handleResponseMimeTypes(response, responseBodyConfigMap);
      console.log('[Worker]', { bodyResponse });

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

      console.log('[Worker]', { output });

      return output;
    },
    setupMixFetch: async (opts) => {
      console.log('[Worker] --- setupMixFetch ---', { opts });
      if (opts?.responseBodyConfigMap) {
        responseBodyConfigMap = opts.responseBodyConfigMap;
      }
      await setupMixFetch(opts || {});
    },
    disconnectMixFetch: async () => {
      console.log('[Worker] --- disconnectMixFetch ---');

      await disconnectMixFetch();
    },
  };

  // start comlink listening for messages and handle them above
  if (parentPort) {
    Comlink.expose(mixFetchWebWorker, nodeEndpoint(parentPort));
  }

  // notify any listeners that the web worker has loaded - HOWEVER, mixFetch hasn't been setup and the client started
  // call `setupMixFetch` from the main thread to start the Nym client
  postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
}
