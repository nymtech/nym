import { invoke } from '@tauri-apps/api';
import { Coin, DelegationResult, EnumNodeType, TauriTxResult, TBondArgs } from '../types';

export const bond = async ({ type, data, pledge, ownerSignature }: TBondArgs): Promise<any> => {
  await invoke(`bond_${type}`, { [type]: data, ownerSignature, pledge });
};

export const unbond = async (type: EnumNodeType): Promise<void> => {
  await invoke(`unbond_${type}`);
};

export const delegate = async ({
  type,
  identity,
  amount,
}: {
  type: EnumNodeType;
  identity: string;
  amount: Coin;
}): Promise<DelegationResult> => {
  const res: DelegationResult = await invoke(`delegate_to_${type}`, { identity, amount });
  return res;
};

export const undelegate = async ({
  type,
  identity,
}: {
  type: EnumNodeType;
  identity: string;
}): Promise<DelegationResult | undefined> => {
  try {
    const res: DelegationResult = await invoke(`undelegate_from_${type}`, { identity });
    return res;
  } catch (e) {
    console.log(e);
    return undefined;
  }
};

export const send = async (args: { amount: Coin; address: string; memo: string }): Promise<TauriTxResult> => {
  const res: TauriTxResult = await invoke('send', args);
  return res;
};

export const updateMixnode = async (profitMarginPercent: number): Promise<void> => {
  await invoke('update_mixnode', { profitMarginPercent });
};
