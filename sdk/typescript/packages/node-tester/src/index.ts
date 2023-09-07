import InlineWasmWebWorker from 'web-worker:./worker';
import * as Comlink from 'comlink';
import { INodeTesterWorkerAsync, NodeTester, NodeTesterEventKinds } from './types';

/**
 * Client for the Nym node tester.
 */
export const createNodeTesterClient = async (): Promise<NodeTester> => {
  // eslint-disable-next-line @typescript-eslint/no-use-before-define
  const worker = await createWorker();

  // let comlink handle interop with the web worker
  const tester = Comlink.wrap<INodeTesterWorkerAsync>(worker);

  // expose the method to terminate the worker
  const terminate = async () => {
    worker.terminate();
  };

  return { tester, terminate };
};

/**
 * Async method to create a web worker that runs the Nym node tester client on another thread. It will only return once the worker
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
        if (msg.data?.kind === NodeTesterEventKinds.Loaded) {
          resolve(worker);
        } else {
          reject(msg);
        }
      },
      { once: true },
    );
  });

export * from './types';
