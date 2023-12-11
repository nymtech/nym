import * as Comlink from 'comlink';
import InlineWasmWebWorker from 'web-worker:./worker';
import {
  EventKinds,
  CredentialClientOpts,
  INymCredentialClientWebWorker,
  NymCredentialsClient,
} from './types';

const mnemonic =
  'summer under connect sadness unveil region charge feed tank grant drift mass side ramp winter fit verb rare huge high garment moment achieve since';

export const createNymCredentialsClient = async (options?: CredentialClientOpts): Promise<NymCredentialsClient> => {
  // eslint-disable-next-line @typescript-eslint/no-use-before-define
  const worker = await createWorker();

  // let comlink handle interop with the web worker
  const comlink = Comlink.wrap<INymCredentialClientWebWorker>(worker);

  return { comlink };
  // pass the client interop and subscription manage back to the caller
  // return {
  //   init(mnemonic) { // TODO: maybe? could also be passed in acquireCred
  //       comlink.setMnemonic(mnemonic);
  //   },
  //   async acquireCredential(coin, mnemonic, options) {
  //       // TODO: wire up promise and event handlers

  //       const handler = (msg: any) => {
  //           if (msg.data && msg.data.kind) {
  //               worker.removeEventListener('message', handler);
  //               return msg.data.credential; // return path in promise
  //           }
  //           worker.removeEventListener('message', handler);
  //       };

  //       worker.addEventListener('message', handler);

  //       // fire message
  //       comlink.acquireCredential(coin);
  //   },
  // };
};

/**
 * Async method to create a web worker that runs the Nym credentials client on another thread. It will only return once the worker
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
