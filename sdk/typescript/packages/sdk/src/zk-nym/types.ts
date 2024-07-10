/**
 * Enum representing various event kinds.
 * @enum
 */
export enum EventKinds {
  Loaded = 'Loaded',
}

export interface LoadedEvent {
  kind: EventKinds.Loaded;
  args: {
    loaded: true;
  };
}

export type ZkNym = any; // TODO

export interface ZkNymClientOpts {
  useSandbox?: boolean;
  networkDetails?: {};
}

export interface INymZkNymClientWebWorker {
  acquireCredential: (coin: string, mnemonic: string, opts: ZkNymClientOpts) => Promise<ZkNym>;
}

export interface NymZkNymClient {
  client: INymZkNymClientWebWorker;
}
