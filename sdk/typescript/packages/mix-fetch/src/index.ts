import InlineWasmWebWorker from 'web-worker:./worker/worker';
import * as Comlink from 'comlink';
import type { IMixFetchWebWorker } from './types';
import { EventKinds } from './types';

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

export const createMixFetch = async (): Promise<IMixFetchWebWorker> => {
  const worker = await createWorker();
  const mixFetchWebWorker = Comlink.wrap<IMixFetchWebWorker>(worker);
  return mixFetchWebWorker;
};

export * from './types';
