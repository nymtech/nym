import { FeeDetails, DecCoin, MixnodeStatus, TransactionExecuteResult } from '@nymproject/types';
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { isGateway, isMixnode, Network, TBondGatewayArgs, TBondMixNodeArgs } from 'src/types';
import { Console } from 'src/utils/console';
import {
  bondGateway as bondGatewayRequest,
  bondMixNode as bondMixNodeRequest,
  claimOperatorReward,
  compoundOperatorReward,
  unbondGateway as unbondGatewayRequest,
  unbondMixNode as unbondMixnodeRequest,
  vestingBondGateway,
  vestingBondMixNode,
  vestingUnbondGateway,
  vestingUnbondMixnode,
  updateMixnode as updateMixnodeRequest,
  vestingUpdateMixnode as updateMixnodeVestingRequest,
  getNodeDescription as getNodeDescriptioRequest,
  getGatewayBondDetails,
  getMixnodeBondDetails,
  getMixnodeStatus,
  getOperatorRewards,
  getMixnodeStakeSaturation,
  getNumberOfMixnodeDelegators,
  vestingClaimOperatorReward,
  vestingCompoundOperatorReward,
} from '../requests';
import { useCheckOwnership } from '../hooks/useCheckOwnership';
import { AppContext } from './main';

const bonded: TBondedMixnode = {
  name: 'Monster node',
  identityKey: 'B2Xx4haarLWMajX8w259oHjtRZsC7nHwagbWrJNiA3QC',
  bond: { denom: 'nym', amount: '1234' },
  delegators: 123,
  operatorRewards: { denom: 'nym', amount: '12' },
  profitMargin: 10,
  stake: { denom: 'nym', amount: '99' },
  stakeSaturation: 99,
  status: 'active',
};

// TODO add relevant data
export type TBondedMixnode = {
  name: string;
  identityKey: string;
  stake: DecCoin;
  bond: DecCoin;
  stakeSaturation: number;
  profitMargin: number;
  operatorRewards: DecCoin;
  delegators: number;
  status: MixnodeStatus;
  proxy?: string;
};

// TODO add relevant data
export interface TBondedGateway {
  name: string;
  identityKey: string;
  ip: string;
  bond: DecCoin;
  location?: string; // TODO not yet available, only available in Network Explorer API
  proxy?: string;
}

export type TokenPool = 'locked' | 'balance';

export type TBondingContext = {
  isLoading: boolean;
  error?: string;
  bondedNode?: TBondedMixnode | TBondedGateway;
  refresh: () => Promise<void>;
  bondMixnode: (data: TBondMixNodeArgs, tokenPool: TokenPool) => Promise<TransactionExecuteResult | undefined>;
  bondGateway: (data: TBondGatewayArgs, tokenPool: TokenPool) => Promise<TransactionExecuteResult | undefined>;
  unbond: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  redeemRewards: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  compoundRewards: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  updateMixnode: (pm: number, fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  checkOwnership: () => Promise<void>;
};

const calculateStake = (pledge: DecCoin, delegations: DecCoin) => {
  const total = Number(pledge.amount) + Number(delegations.amount);
  return { amount: total.toString(), denom: pledge.denom };
};

export const BondingContext = createContext<TBondingContext>({
  isLoading: true,
  refresh: async () => undefined,
  bondMixnode: async () => {
    throw new Error('Not implemented');
  },
  bondGateway: async () => {
    throw new Error('Not implemented');
  },
  unbond: async () => {
    throw new Error('Not implemented');
  },
  redeemRewards: async () => {
    throw new Error('Not implemented');
  },
  compoundRewards: async () => {
    throw new Error('Not implemented');
  },
  updateMixnode: async () => {
    throw new Error('Not implemented');
  },
  checkOwnership(): Promise<void> {
    throw new Error('Not implemented');
  },
});

export const BondingContextProvider = ({ children }: { children?: React.ReactNode }): JSX.Element => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [bondedNode, setBondedNode] = useState<TBondedMixnode | TBondedGateway>();

  const { userBalance, clientDetails } = useContext(AppContext);
  const { ownership, isLoading: isOwnershipLoading, checkOwnership } = useCheckOwnership();

  const isVesting = Boolean(ownership.vestingPledge);

  const resetState = () => {
    setError(undefined);
    setBondedNode(undefined);
  };

  const getAdditionalMixnodeDetails = async (identityKey: string) => {
    const additionalDetails: { status: MixnodeStatus; stakeSaturation: number; numberOfDelegators: number } = {
      status: 'not_found',
      stakeSaturation: 0,
      numberOfDelegators: 0,
    };

    try {
      const statusResponse = await getMixnodeStatus(identityKey);
      additionalDetails.status = statusResponse.status;
    } catch (e) {
      Console.log(e);
    }

    try {
      const stakeSaturationResponse = await getMixnodeStakeSaturation(identityKey);
      additionalDetails.stakeSaturation = Math.round(stakeSaturationResponse.saturation * 100);
    } catch (e) {
      Console.log(e);
    }

    try {
      const numberOfDelegators = await getNumberOfMixnodeDelegators(identityKey);
      additionalDetails.numberOfDelegators = numberOfDelegators;
    } catch (e) {
      Console.log(e);
    }

    return additionalDetails;
  };

  const getNodeDescription = async (host: string, port: number) => {
    try {
      const nodeDescription = await getNodeDescriptioRequest(host, port);
      return nodeDescription;
    } catch (e) {
      Console.log(e);
    }
    return undefined;
  };

  const refresh = useCallback(async () => {
    setIsLoading(true);

    if (ownership.hasOwnership && ownership.nodeType === 'mixnode' && clientDetails) {
      try {
        const data = await getMixnodeBondDetails();
        const operatorRewards = await getOperatorRewards(clientDetails?.client_address);
        if (data) {
          const { status, stakeSaturation, numberOfDelegators } = await getAdditionalMixnodeDetails(
            data.mix_node.identity_key,
          );
          const nodeDescription = await getNodeDescription(data.mix_node.host, data.mix_node.http_api_port);
          setBondedNode({
            name: nodeDescription?.name || '-',
            identityKey: data.mix_node.identity_key,
            ip: '',
            stake: calculateStake(data.pledge_amount, data.total_delegation),
            bond: data.pledge_amount,
            profitMargin: data.mix_node.profit_margin_percent,
            nodeRewards: data.accumulated_rewards,
            delegators: numberOfDelegators,
            proxy: data.proxy,
            operatorRewards,
            status,
            stakeSaturation,
          } as TBondedMixnode);
        }
      } catch (e: any) {
        setError(`While fetching current bond state, an error occurred: ${e}`);
      }
    }

    if (ownership.hasOwnership && ownership.nodeType === 'gateway') {
      try {
        const data = await getGatewayBondDetails();
        if (data) {
          const nodeDescription = await getNodeDescription(data.gateway.host, data.gateway.clients_port);

          setBondedNode({
            name: nodeDescription?.name || '-',
            identityKey: data.gateway.identity_key,
            ip: data.gateway.host,
            location: data.gateway.location,
            bond: data.pledge_amount,
            delegators: bonded.delegators,
            proxy: data.proxy,
          } as TBondedGateway);
        }
      } catch (e: any) {
        setError(`While fetching current bond state, an error occurred: ${e}`);
      }
    }

    if (!ownership.hasOwnership) {
      resetState();
    }
    setIsLoading(false);
  }, [ownership]);

  useEffect(() => {
    refresh();
  }, [ownership, refresh]);

  const bondMixnode = async (data: TBondMixNodeArgs, tokenPool: TokenPool) => {
    let tx: TransactionExecuteResult | undefined;
    setIsLoading(true);
    try {
      if (tokenPool === 'balance') {
        tx = await bondMixNodeRequest(data);
        await userBalance.fetchBalance();
      }
      if (tokenPool === 'locked') {
        tx = await vestingBondMixNode(data);
        await userBalance.fetchTokenAllocation();
      }
      return tx;
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return undefined;
  };

  const bondGateway = async (data: TBondGatewayArgs, tokenPool: TokenPool) => {
    let tx: TransactionExecuteResult | undefined;
    setIsLoading(true);
    try {
      if (tokenPool === 'balance') {
        tx = await bondGatewayRequest(data);
        await userBalance.fetchBalance();
      }
      if (tokenPool === 'locked') {
        tx = await vestingBondGateway(data);
        await userBalance.fetchTokenAllocation();
      }
      return tx;
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return undefined;
  };

  const unbond = async (fee?: FeeDetails) => {
    let tx;
    setIsLoading(true);
    try {
      if (bondedNode && isMixnode(bondedNode) && bondedNode.proxy) tx = await vestingUnbondMixnode(fee?.fee);
      if (bondedNode && isMixnode(bondedNode) && !bondedNode.proxy) tx = await unbondMixnodeRequest(fee?.fee);
      if (bondedNode && isGateway(bondedNode) && bondedNode.proxy) tx = await vestingUnbondGateway(fee?.fee);
      if (bondedNode && isGateway(bondedNode) && !bondedNode.proxy) tx = await unbondGatewayRequest(fee?.fee);
    } catch (e) {
      setError(`an error occurred: ${e as string}`);
    } finally {
      setIsLoading(false);
    }
    return tx;
  };

  const updateMixnode = async (pm: number, fee?: FeeDetails) => {
    let tx;
    setIsLoading(true);
    try {
      if (bondedNode?.proxy) tx = await updateMixnodeVestingRequest(pm, fee?.fee);
      else tx = await updateMixnodeRequest(pm, fee?.fee);
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return tx;
  };

  const redeemRewards = async (fee?: FeeDetails) => {
    let tx;
    setIsLoading(true);
    try {
      if (bondedNode?.proxy) tx = await vestingClaimOperatorReward(fee?.fee);
      else tx = await claimOperatorReward(fee?.fee);
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return tx;
  };

  const compoundRewards = async (fee?: FeeDetails) => {
    let tx;
    setIsLoading(true);
    try {
      if (bondedNode?.proxy) tx = await vestingCompoundOperatorReward(fee?.fee);
      else tx = await compoundOperatorReward(fee?.fee);
    } catch (e: any) {
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return tx;
  };

  const bondMore = async (_signature: string, _additionalBond: DecCoin) =>
    // TODO to implement
    undefined;

  const memoizedValue = useMemo(
    () => ({
      isLoading: isLoading || isOwnershipLoading,
      error,
      bondMixnode,
      bondedNode,
      bondGateway,
      unbond,
      updateMixnode,
      refresh,
      redeemRewards,
      compoundRewards,
      bondMore,
      checkOwnership,
    }),
    [isLoading, isOwnershipLoading, error, bondedNode, isVesting],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
