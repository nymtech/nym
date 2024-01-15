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

export type Credential = any; // TODO

export interface CredentialClientOpts {
  useSandbox?: boolean;
  networkDetails?: {};
}

export interface INymCredentialClientWebWorker {
  acquireCredential: (coin: string, mnemonic: string, opts: CredentialClientOpts) => Promise<Credential>;
}

// export interface NymCredentialsClient {
//   init: (mnemonic: string) => void;
//   acquireCredential: (coin: Coin, mnemonic: string, options?: CredentialsClientOpts) => Promise<Credentials>;
// }

export interface NymCredentialsClient {
  comlink: INymCredentialClientWebWorker;
}
