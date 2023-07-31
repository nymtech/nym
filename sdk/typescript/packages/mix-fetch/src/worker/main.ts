/* eslint-disable no-restricted-globals */
import { MixFetchConfig, no_cover_debug, setupMixFetch } from '@nymproject/mix-fetch-wasm';
import * as Comlink from 'comlink';
import type { IMixFetchWebWorker, LoadedEvent } from '../types';
import { EventKinds } from '../types';

/**
 * Helper method to send typed messages.
 * @param event   The strongly typed message to send back to the calling thread.
 */
// eslint-disable-next-line no-restricted-globals
const postMessageWithType = <E>(event: E) => self.postMessage(event);
export async function run() {
  const { mixFetch } = self as any;

  const mixFetchWebWorker: IMixFetchWebWorker = {
    mixFetch: async (url, args) => {
      console.log('[Worker] --- mixFetch ---', { url, args });

      const response: Response = await mixFetch(url, args);

      console.log('[Worker]', { response, json: JSON.stringify(response, null, 2) });

      let body;
      let json;
      let text;
      let formData;
      let blob;

      try {
        text = await response.clone().text();
      } catch (e: any) {
        console.warn('text', e);
      }

      console.log('[Worker]', { body, text });

      const headers: any = {};
      response.headers.forEach((value, key) => {
        headers[key] = value;
      });

      console.log('[Worker]', { headers, json, text, formData, blob });

      const output = {
        body,
        url: response.url,
        headers,
        status: response.status,
        statusText: response.statusText,
        type: response.type,
        ok: response.ok,
        redirected: response.redirected,
        json,
        text,
        formData,
        blob,
      };

      console.log('[Worker]', { output });

      return output;
    },
    setupMixFetch: async (opts) => {
      console.log('[Worker] --- setupMixFetch ---', { opts });
      await setupMixFetch(opts);
    },
  };

  // start comlink listening for messages and handle them above
  Comlink.expose(mixFetchWebWorker);

  // notify any listeners that the web worker has loaded - HOWEVER, mixFetch hasn't been setup and the client started
  // call `setupMixFetch` from the main thread to start the Nym client
  postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
}
