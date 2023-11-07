/* eslint-disable-next-line no-console */

import * as Comlink from 'comlink';
import InlineWasmWebWorker from 'web-worker:./worker/worker';
import { Worker } from 'node:worker_threads';

import nodeEndpoint from './node-adapter';
import type { IMixFetchWebWorker } from './types';
import { EventKinds, IMixFetch } from './types';

const createWorker = async () =>
  new Promise<Worker>((resolve, reject) => {
    // rollup will inline the built worker script, so that when the SDK is used in
    // other projects, they will not need to mess around trying to bundle it
    // however, it will make this SDK bundle bigger because of Base64 inline data
    const worker = new InlineWasmWebWorker();

    worker.addListener('error', reject);
    worker.addListener('message', (msg: any) => {
      worker.removeListener('error', reject);
      if (msg.kind === EventKinds.Loaded) {
        resolve(worker);
      } else {
        reject(msg);
      }
    });
  });

const convertHeaders = (headers: any): Headers => {
  const out = new Headers();
  Object.keys(headers).forEach((key) => {
    out.append(key, headers[key]);
  });
  return out;
};

/**
 * Use this method to initialise `mixFetch`.
 *
 * @returns An instance of `mixFetch` that you can use to make your requests using the same interface as `fetch`.
 */
export const createMixFetch = async (): Promise<IMixFetch> => {
  // start the worker
  const worker = await createWorker();

  // bind with Comlink
  const wrappedWorker = Comlink.wrap<IMixFetchWebWorker>(nodeEndpoint(worker));

  // handle the responses
  const mixFetchWebWorker: IMixFetch = {
    setupMixFetch: wrappedWorker.setupMixFetch,
    mixFetch: async (url: string, args: any) => {
      const workerResponse = await wrappedWorker.mixFetch(url, args);
      if (!workerResponse) {
        throw new Error('No response received');
      }
      const { headers: headersRaw, status, statusText } = workerResponse;

      // reconstruct the Headers object instance from a plain object
      const headers = convertHeaders(headersRaw);

      // handle blobs
      if (workerResponse.body.blobUrl) {
        const blob = await (await fetch(workerResponse.body.blobUrl)).blob();
        const body = await blob.arrayBuffer();
        return new Response(body, { headers, status, statusText });
      }

      // handle everything else
      const body = Object.values(workerResponse.body)[0]; // we are expecting only one value to be set in `.body`
      return new Response(body, { headers, status, statusText });
    },
    disconnectMixFetch: wrappedWorker.disconnectMixFetch,
  };

  return mixFetchWebWorker;
};
