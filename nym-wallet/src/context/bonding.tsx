import {
  FeeDetails,
  DecCoin,
  MixnodeStatus,
  TransactionExecuteResult,
  decimalToFloatApproximation,
  decimalToPercentage,
} from '@nymproject/types';
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { isGateway, isMixnode, TBondGatewayArgs, TBondMixNodeArgs } from 'src/types';
import { Console } from 'src/utils/console';
import {
  bondGateway as bondGatewayRequest,
  bondMixNode as bondMixNodeRequest,
  claimOperatorReward,
  unbondGateway as unbondGatewayRequest,
  unbondMixNode as unbondMixnodeRequest,
  vestingBondGateway,
  vestingBondMixNode,
  vestingUnbondGateway,
  vestingUnbondMixnode,
  updateMixnodeCostParams as updateMixnodeCostParamsRequest,
  vestingUpdateMixnodeCostParams as updateMixnodeVestingCostParamsRequest,
  getNodeDescription as getNodeDescriptioRequest,
  getGatewayBondDetails,
  getMixnodeBondDetails,
  getMixnodeStatus,
  getPendingOperatorRewards,
  getMixnodeStakeSaturation,
  vestingClaimOperatorReward,
} from '../requests';
import { useCheckOwnership } from '../hooks/useCheckOwnership';
import { AppContext } from './main';
import { attachDefaultOperatingCost, toPercentFloatString, toPercentIntegerString } from '../utils';

// TODO add relevant data
export type TBondedMixnode = {
  name?: string;
  identityKey: string;
  stake: DecCoin;
  bond: DecCoin;
  stakeSaturation: string;
  profitMargin: string;
  operatorRewards?: DecCoin;
  delegators: number;
  status: MixnodeStatus;
  proxy?: string;
  host: string;
  httpApiPort: number;
  mixPort: number;
  verlocPort: number;
  version: string;
};

export interface TBondedGateway {
  name: string;
  identityKey: string;
  ip: string;
  bond: DecCoin;
  location?: string; // TODO not yet available, only available in Network Explorer API
  proxy?: string;
  host: string;
  httpApiPort: number;
  mixPort: number;
  profitMargin: string;
  verlocPort: number;
  version: string;
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
  updateMixnode: (pm: string, fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  checkOwnership: () => Promise<void>;
};

const calculateStake = (pledge: string, delegations: string): number =>
  decimalToFloatApproximation(pledge) + decimalToFloatApproximation(delegations);

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

  const getAdditionalMixnodeDetails = async (mixId: number) => {
    const additionalDetails: { status: MixnodeStatus; stakeSaturation: string } = {
      status: 'not_found',
      stakeSaturation: '0',
    };
    try {
      const statusResponse = await getMixnodeStatus(mixId);
      additionalDetails.status = statusResponse.status;
    } catch (e) {
      Console.log(e);
    }

    try {
      const stakeSaturationResponse = await getMixnodeStakeSaturation(mixId);
      additionalDetails.stakeSaturation = decimalToPercentage(stakeSaturationResponse.saturation);
    } catch (e) {
      Console.log(e);
    }
    return additionalDetails;
  };

  const getNodeDescription = async (host: string, port: number) => {
    try {
      return await getNodeDescriptioRequest(host, port);
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
        let operatorRewards;
        try {
          operatorRewards = await getPendingOperatorRewards(clientDetails?.client_address);
        } catch (e) {
          Console.warn(`get_operator_rewards request failed: ${e}`);
        }
        if (data) {
          const { status, stakeSaturation } = await getAdditionalMixnodeDetails(data.bond_information.id);
          const nodeDescription = await getNodeDescription(
            data.bond_information.mix_node.host,
            data.bond_information.mix_node.http_api_port,
          );
          setBondedNode({
            name: nodeDescription?.name,
            identityKey: data.bond_information.mix_node.identity_key,
            ip: '',
            stake: {
              amount: calculateStake(data.rewarding_details.operator, data.rewarding_details.delegates).toString(),
              denom: data.bond_information.original_pledge.denom,
            },
            bond: data.bond_information.original_pledge,
            profitMargin: toPercentIntegerString(data.rewarding_details.cost_params.profit_margin_percent),
            delegators: data.rewarding_details.unique_delegations,
            proxy: data.bond_information.proxy,
            operatorRewards,
            status,
            stakeSaturation,
            host: data.bond_information.mix_node.host.replace(/\s/g, ''),
            httpApiPort: data.bond_information.mix_node.http_api_port,
            mixPort: data.bond_information.mix_node.mix_port,
            verlocPort: data.bond_information.mix_node.verloc_port,
            version: data.bond_information.mix_node.version,
          } as TBondedMixnode);
        }
      } catch (e: any) {
        Console.warn(e);
        setError(`While fetching current bond state, an error occurred: ${e}`);
      }
    }

    if (ownership.hasOwnership && ownership.nodeType === 'gateway') {
      try {
        const data = await getGatewayBondDetails();
        if (data) {
          const nodeDescription = await getNodeDescription(data.gateway.host, data.gateway.clients_port);

          setBondedNode({
            name: nodeDescription?.name,
            identityKey: data.gateway.identity_key,
            ip: data.gateway.host,
            location: data.gateway.location,
            bond: data.pledge_amount,
            proxy: data.proxy,
          } as TBondedGateway);
        }
      } catch (e: any) {
        Console.warn(e);
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
      Console.warn(e);
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
      Console.warn(e);
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
      Console.warn(e);
      setError(`an error occurred: ${e as string}`);
    } finally {
      setIsLoading(false);
    }
    return tx;
  };

  const updateMixnode = async (pm: string, fee?: FeeDetails) => {
    let tx;
    setIsLoading(true);

    // TODO: this will have to be updated with allowing users to provide their operating cost in the form
    const defaultCostParams = await attachDefaultOperatingCost(toPercentFloatString(pm));

    try {
      // JS: this check is not entirely valid. you can have proxy field set whilst not using the vesting contract,
      // you have to check if proxy exists AND if it matches the known vesting contract address!
      if (bondedNode?.proxy) {
        tx = await updateMixnodeVestingCostParamsRequest(defaultCostParams, fee?.fee);
      } else {
        tx = await updateMixnodeCostParamsRequest(defaultCostParams, fee?.fee);
      }
    } catch (e: any) {
      Console.warn(e);
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
      bondMore,
      checkOwnership,
    }),
    [isLoading, isOwnershipLoading, error, bondedNode, isVesting],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
