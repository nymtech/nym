import React, { createContext, useCallback, useContext, useMemo, useState } from 'react';
import { Delegation, PendingEpochEvent, PendingEpochEventKind } from '@nymproject/contract-clients/Mixnet.types';
import { ExecuteResult } from '@cosmjs/cosmwasm-stargate';
import { useWalletContext } from './wallet';
import { useMainContext } from './main';

const fee = { gas: '1000000', amount: [{ amount: '1000000', denom: 'unym' }] };

export type PendingEvent = ReturnType<typeof getEventsByAddress>;

export type DelegationWithRewards = Delegation & {
  rewards: string;
  identityKey: string;
  pending: PendingEvent;
};

const getEventsByAddress = (kind: PendingEpochEventKind, address: String) => {
  if ('delegate' in kind && kind.delegate.owner === address) {
    return {
      kind: 'delegate' as const,
      mixId: kind.delegate.mix_id,
      amount: kind.delegate.amount,
    };
  }

  if ('undelegate' in kind && kind.undelegate.owner === address) {
    return {
      kind: 'undelegate' as const,
      mixId: kind.undelegate.mix_id,
    };
  }

  return undefined;
};

interface DelegationsState {
  delegations?: DelegationWithRewards[];
  handleGetDelegations: () => Promise<void>;
  handleDelegate: (mixId: number, amount: string) => Promise<ExecuteResult | undefined>;
  handleUndelegate: (mixId: number) => Promise<ExecuteResult | undefined>;
}

export const DelegationsContext = createContext<DelegationsState>({
  delegations: undefined,
  handleGetDelegations: async () => {
    throw new Error('Please connect your wallet');
  },
  handleDelegate: async () => {
    throw new Error('Please connect your wallet');
  },
  handleUndelegate: async () => {
    throw new Error('Please connect your wallet');
  },
});

export const DelegationsProvider = ({ children }: { children: React.ReactNode }) => {
  const [delegations, setDelegations] = useState<DelegationWithRewards[]>();
  const { address, nymQueryClient, nymClient } = useWalletContext();
  const { fetchMixnodes } = useMainContext();

  const handleGetPendingEvents = async () => {
    if (!nymQueryClient) {
      return undefined;
    }

    if (!address) {
      return undefined;
    }

    const response = await nymQueryClient.getPendingEpochEvents({});
    const pendingEvents: PendingEvent[] = [];

    response.events.forEach((e: PendingEpochEvent) => {
      const event = getEventsByAddress(e.event.kind, address);
      if (event) {
        pendingEvents.push(event);
      }
    });

    return pendingEvents;
  };

  const handleGetDelegationRewards = async (mixId: number) => {
    if (!nymQueryClient) {
      return undefined;
    }

    if (!address) {
      return undefined;
    }

    const response = await nymQueryClient.getPendingDelegatorReward({ address, mixId });

    return response;
  };

  const handleGetDelegations = useCallback(async () => {
    if (!nymQueryClient) {
      setDelegations(undefined);
      return undefined;
    }

    if (!address) {
      setDelegations(undefined);
      return undefined;
    }

    // Get all mixnodes - Required to get the identity key for each delegation
    const mixnodes = await fetchMixnodes();

    // Get delegations
    const delegationsResponse = await nymQueryClient.getDelegatorDelegations({ delegator: address });

    // Get rewards for each delegation
    const rewardsResponse = await Promise.all(
      delegationsResponse.delegations.map((d: Delegation) => handleGetDelegationRewards(d.mix_id)),
    );

    // Get all pending events
    const pendingEvents = await handleGetPendingEvents();

    const delegationsWithRewards: DelegationWithRewards[] = [];

    // Merge delegations with rewards and pending events
    delegationsResponse.delegations.forEach((d: Delegation, index: number) => {
      delegationsWithRewards.push({
        ...d,
        pending: pendingEvents?.find((e: PendingEvent) => (e?.mixId === d.mix_id ? e.kind : undefined)),
        identityKey: mixnodes?.find((m) => m.mix_id === d.mix_id)?.mix_node.identity_key || '',
        rewards: rewardsResponse[index]?.amount_earned_detailed || '0',
      });
    });

    // Add pending events that are not in the delegations list
    pendingEvents?.forEach((e) => {
      if (e && !delegationsWithRewards.find((d: DelegationWithRewards) => d.mix_id === e.mixId)) {
        delegationsWithRewards.push({
          mix_id: e.mixId,
          height: 0,
          cumulative_reward_ratio: '0',
          owner: address,
          amount: {
            amount: '0',
            denom: 'unym',
          },
          rewards: '0',
          identityKey: mixnodes?.find((m) => m.mix_id === e.mixId)?.mix_node.identity_key || '',
          pending: e,
        });
      }
    });

    setDelegations(delegationsWithRewards);

    return undefined;
  }, [address, nymQueryClient]);

  const handleDelegate = async (mixId: number, amount: string) => {
    if (!address) {
      throw new Error('Please connect your wallet');
    }

    const amountToDelegate = (Number(amount) * 1000000).toString();
    const uNymFunds = [{ amount: amountToDelegate, denom: 'unym' }];
    try {
      const tx = await nymClient?.delegateToMixnode({ mixId }, fee, 'Delegation from Nym Explorer', uNymFunds);

      return tx as unknown as ExecuteResult;
    } catch (e) {
      console.error('Failed to delegate to mixnode', e);
      throw e;
    }
  };

  const handleUndelegate = async (mixId: number) => {
    const tx = await nymClient?.undelegateFromMixnode({ mixId }, fee);

    return tx as unknown as ExecuteResult;
  };

  const contextValue: DelegationsState = useMemo(
    () => ({
      delegations,
      handleGetDelegations,
      handleDelegate,
      handleUndelegate,
    }),
    [delegations, handleGetDelegations],
  );

  return <DelegationsContext.Provider value={contextValue}>{children}</DelegationsContext.Provider>;
};

export const useDelegationsContext = () => {
  const context = useContext(DelegationsContext);
  if (!context) {
    throw new Error('useDelegationsContext must be used within a DelegationsProvider');
  }
  return context;
};
