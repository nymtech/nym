import { invoke } from '@tauri-apps/api';
import { Coin, DelegationResult, EnumNodeType, Gateway, MixNode, TauriTxResult } from '../types';

export const bond = async ({
  type,
  data,
  pledge,
  ownerSignature,
}: {
  type: EnumNodeType;
  data: MixNode | Gateway;
  pledge: Coin;
  ownerSignature: string;
}): Promise<any> => invoke(`bond_${type}`, { [type]: data, ownerSignature, pledge });

export const unbond = async (type: EnumNodeType) => invoke(`unbond_${type}`);

export const delegate = async ({
  type,
  identity,
  amount,
}: {
  type: EnumNodeType;
  identity: string;
  amount: Coin;
}): Promise<DelegationResult> => invoke(`delegate_to_${type}`, { identity, amount });

export const undelegate = async ({
  type,
  identity,
}: {
  type: EnumNodeType;
  identity: string;
}): Promise<DelegationResult> => invoke(`undelegate_from_${type}`, { identity });

export const send = async (args: { amount: Coin; address: string; memo: string }): Promise<TauriTxResult> =>
  invoke('send', args);

export const updateMixnode = async ({ profitMarginPercent }: { profitMarginPercent: number }) =>
  invoke('update_mixnode', { profitMarginPercent });
