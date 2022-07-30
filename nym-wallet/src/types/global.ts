import { Gateway, DecCoin, MixNode, PledgeData } from '@nymproject/types';
import { Fee } from '@nymproject/types/dist/types/rust/Fee';
import { TBondedGateway, TBondedMixnode } from 'src/context';

export enum EnumNodeType {
  mixnode = 'mixnode',
  gateway = 'gateway',
}

export type TNodeOwnership = {
  hasOwnership: boolean;
  nodeType?: EnumNodeType;
  vestingPledge?: PledgeData;
};

export type TPendingDelegation = {
  block_height: number;
};

export type TDelegation = {
  owner: string;
  node_identity: string;
  amount: DecCoin;
  block_height: number;
  proxy: string; // proxy address used to delegate the funds on behalf of another address
  pending?: TPendingDelegation;
};

export type TBondGatewayArgs = {
  gateway: Gateway;
  pledge: DecCoin;
  ownerSignature: string;
  fee?: Fee;
};

export type TBondMixNodeArgs = {
  mixnode: MixNode;
  pledge: DecCoin;
  ownerSignature: string;
  fee?: Fee;
};

export type TNodeDescription = {
  name: string;
  description: string;
  link: string;
  location: string;
};

export type TDelegateArgs = {
  identity: string;
  amount: DecCoin;
};

export type Period = 'Before' | { In: number } | 'After';

export type TAccount = {
  name: string;
  address: string;
  mnemonic: string;
};

export const isMixnode = (node: TBondedMixnode | TBondedGateway): node is TBondedMixnode =>
  (node as TBondedMixnode).profitMargin !== undefined;

export const isGateway = (node: TBondedMixnode | TBondedGateway): node is TBondedGateway => !isMixnode(node);
