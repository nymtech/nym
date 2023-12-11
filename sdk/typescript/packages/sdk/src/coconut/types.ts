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

export interface Coin {
  amount: string;
  denom: string;
}

export type Credentials = any; // TODO

export interface CredentialsClientOpts {
  isSandbox?: boolean;
}

export interface INymCredentialsClientWebWorker {
  acquireCredential: (coin: Coin, mnemonic: string, opts: CredentialsClientOpts) => Promise<Credential>;
}

// export interface NymCredentialsClient {
//   init: (mnemonic: string) => void;
//   acquireCredential: (coin: Coin, mnemonic: string, options?: CredentialsClientOpts) => Promise<Credentials>;
// }

export interface NymCredentialsClient {
  comlink: INymCredentialsClientWebWorker;
}
