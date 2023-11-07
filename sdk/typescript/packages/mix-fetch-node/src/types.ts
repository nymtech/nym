import type { MixFetchOpts } from '@nymproject/mix-fetch-wasm-node';

type IMixFetchWorkerFn = (url: string, args: any) => Promise<MixFetchWebWorkerResponse>;

// export type IMixFetchFn = typeof fetch;
export type IMixFetchFn = (url: string, args: any, opts?: SetupMixFetchOps) => Promise<Response>;

export type SetupMixFetchOps = MixFetchOpts & {
  responseBodyConfigMap?: ResponseBodyConfigMap;
};

export interface IMixFetchWebWorker {
  mixFetch: IMixFetchWorkerFn;
  setupMixFetch: (opts?: SetupMixFetchOps) => Promise<void>;
  disconnectMixFetch: () => Promise<void>;
}

export interface IMixFetch {
  mixFetch: IMixFetchFn;
  setupMixFetch: (opts?: SetupMixFetchOps) => Promise<void>;
  disconnectMixFetch: () => Promise<void>;
}

export enum EventKinds {
  Loaded = 'Loaded',
}

export interface LoadedEvent {
  kind: EventKinds.Loaded;
  args: {
    loaded: true;
  };
}

export interface ResponseBody {
  uint8array?: Uint8Array;
  json?: any;
  text?: string;
  formData?: any;
  blobUrl?: string;
}

export type ResponseBodyMethod = 'uint8array' | 'json' | 'text' | 'formData' | 'blob';

export interface ResponseBodyConfigMap {
  /**
   * Set the response `Content-Type`s to decode as uint8array.
   */
  uint8array?: Array<RegExp | string>;

  /**
   * Set the response `Content-Type`s to decode with the `json()` response body method.
   */
  json?: Array<RegExp | string>;

  /**
   * Set the response `Content-Type`s to decode with the `text()` response body method.
   */
  text?: Array<RegExp | string>;

  /**
   * Set the response `Content-Type`s to decode with the `formData()` response body method.
   */
  formData?: Array<RegExp | string>;

  /**
   * Set the response `Content-Type`s to decode with the `blob()` response body method.
   */
  blob?: Array<RegExp | string>;
  /**
   * Set this to the default fallback method. Set to `undefined` if you want to ignore unknown types.
   */

  fallback?: ResponseBodyMethod;
}

/**
 * Default values for the handling of response bodies.
 */
export const ResponseBodyConfigMapDefaults: ResponseBodyConfigMap = {
  uint8array: ['application/octet-stream'],
  json: ['application/json', 'text/json', /application\/json.*/, /text\/json\+.*/],
  text: ['text/plain', /text\/plain.*/, 'text/html', /text\/html.*/],
  formData: ['application/x-www-form-urlencoded', 'multipart/form-data'],
  blob: [/image\/.*/, /video\/.*/],
  fallback: 'blob',
};

export interface MixFetchWebWorkerResponse {
  body: ResponseBody;
  url: string;
  headers: any;
  status: number;
  statusText: string;
  type: string;
  ok: boolean;
  redirected: boolean;
}
