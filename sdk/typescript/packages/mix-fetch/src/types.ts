import type { MixFetchOpts } from '@nymproject/mix-fetch-wasm';

// type IMixFetch = typeof fetch;
type IMixFetch = (url: string, args: any) => Promise<any>;

export interface IMixFetchWebWorker {
  mixFetch: IMixFetch;
  setupMixFetch: (network_requester_address: string, opts: MixFetchOpts) => Promise<void>;
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
