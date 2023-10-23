/* eslint-disable no-underscore-dangle */
import type { SetupMixFetchOps, IMixFetchFn } from './types';
import { createMixFetch as createMixFetchInternal } from './create-mix-fetch';

// this is the default timeout for getting a response
const REQUEST_TIMEOUT_MILLISECONDS = 60_000;

export * from './types';

/**
 * Create a global mixFetch instance and optionally configure settings.
 *
 * @param opts Optional settings
 */
export const createMixFetch = async (opts?: SetupMixFetchOps) => {
  if (!(globalThis as any).__mixFetchGlobal) {
    // load the worker and set up mixFetch with defaults
    (globalThis as any).__mixFetchGlobal = await createMixFetchInternal();
    await (globalThis as any).__mixFetchGlobal.setupMixFetch(opts);
  }

  return (globalThis as any).__mixFetchGlobal;
};

/**
 * mixFetch is a drop-in replacement for the standard `fetch` interface.
 *
 * @param url  The URL to fetch from.
 * @param args Fetch options.
 * @param opts Optionally configure mixFetch when it gets created. This only happens once, the first time it gets used.
 */
export const mixFetch: IMixFetchFn = async (url, args, opts?: SetupMixFetchOps) => {
  // ensure mixFetch instance exists
  const instance = await createMixFetch({
    mixFetchOverride: {
      requestTimeoutMs: REQUEST_TIMEOUT_MILLISECONDS,
    },
    ...opts,
  });

  // execute user request
  return instance.mixFetch(url, args);
};

/**
 * Stops the usage of mixFetch and disconnect the client from the mixnet.
 */
export const disconnectMixFetch = async (): Promise<void> => {
  // JS: I'm ignoring this lint (no-else-return) because I want to explicitly state
  // that `__mixFetchGlobal` is definitely not null in the else branch.
  if (!(globalThis as any).__mixFetchGlobal) {
    throw new Error("mixFetch hasn't been setup");
  } else {
    return (globalThis as any).__mixFetchGlobal.disconnectMixFetch();
  }
};
