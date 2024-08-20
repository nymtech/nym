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

export interface ZkNymFaucetClientOpts {
  useSandbox?: boolean;
  networkDetails?: {};
}

export interface INymZkNymFaucetClientWebWorker {
  acquireCredential: (faucetApiUrl: string, authToken: string) => Promise<ZkNym>;
}

export interface NymZkNymFaucetClient {
  client: INymZkNymFaucetClientWebWorker;
}
