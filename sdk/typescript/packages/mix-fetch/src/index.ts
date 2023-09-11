/* eslint-disable no-underscore-dangle */
import type { SetupMixFetchOps, IMixFetchFn, IMixFetch } from './types';
import { createMixFetch as createMixFetchInternal } from './create-mix-fetch';

export * from './types';

declare global {
  interface Window {
    /**
     * Keep a singleton of the mixFetch interface on the `window` object.
     */
    __mixFetchGlobal?: IMixFetch;
  }
}

/**
 * Create a global mixFetch instance and optionally configure settings.
 *
 * @param opts Optional settings
 */
export const createMixFetch = async (opts?: SetupMixFetchOps) => {
  if (!window.__mixFetchGlobal) {
    // load the worker and set up mixFetch with defaults
    window.__mixFetchGlobal = await createMixFetchInternal();
    await window.__mixFetchGlobal.setupMixFetch(opts);
  }
  return window.__mixFetchGlobal;
};

/**
 * mixFetch is a drop-in replacement for the standard `fetch` interface.
 *
 * @param url  The URL to fetch from.
 * @param args Fetch options.
 */
export const mixFetch: IMixFetchFn = async (url, args) => {
  // ensure mixFetch instance exists
  const instance = await createMixFetch({
    mixFetchOverride: {
      requestTimeoutMs: 30_000,
    },
  });

  // execute user request
  return instance.mixFetch(url, args);
};
