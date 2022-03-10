import { Coin, Denom, MixNode } from './rust';

export enum EnumNodeType {
  mixnode = 'mixnode',
  gateway = 'gateway',
}

export type TNodeOwnership = {
  hasOwnership: boolean;
  nodeType?: EnumNodeType;
};

export type TClientDetails = {
  account: {
    client_address: string;
    contract_address: string;
    demon: Denom;
  };
};

export type TSignInWithMnemonic = {
  denom: string;
} & TClientDetails;

export type TCreateAccount = {
  mnemonic: string;
} & TSignInWithMnemonic;

export type TFee = {
  [EnumNodeType.mixnode]: Coin;
  [EnumNodeType.gateway]?: Coin;
};

export type TDelegation = {
  owner: string;
  node_identity: string;
  amount: Coin;
  block_height: number;
  proxy: string; // proxy address used to delegate the funds on behalf of anouther address
};

export type TPagedDelegations = {
  delegations: TDelegation[];
  start_next_after: string;
};

export type TMixnodeBondDetails = {
  pledge_amount: Coin;
  total_delegation: Coin;
  owner: string;
  layer: string;
  block_height: number;
  mix_node: MixNode;
  proxy: any;
};

export type TCurrency = {
  minor: 'UNYM' | 'UNYMT';
  major: 'NYM' | 'NYMT';
};

export type Period = 'Before' | { In: number } | 'After';
