import { invoke } from '@tauri-apps/api';
import { Coin, DelegationResult, EnumNodeType, Gateway, MixNode, TauriTxResult, TBondArgs } from '../types';

export const bond = async ({ type, data, pledge, ownerSignature }: TBondArgs): Promise<any> =>
  await invoke(`bond_${type}`, { [type]: data, ownerSignature, pledge });

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
}): Promise<DelegationResult | undefined> => {
  try {
    return await invoke(`undelegate_from_${type}`, { identity });
  } catch (e) {
    console.log(e);
    return undefined;
  }
};

export const send = async (args: { amount: Coin; address: string; memo: string }): Promise<TauriTxResult> =>
  invoke('send', args);

export const updateMixnode = async (profitMarginPercent: number) =>
  await invoke('update_mixnode', { profitMarginPercent });
