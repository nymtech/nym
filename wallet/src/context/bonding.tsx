import {
  FeeDetails,
  DecCoin,
  MixnodeStatus,
  TransactionExecuteResult,
  decimalToPercentage,
  SelectionChance,
  InclusionProbabilityResponse,
  decimalToFloatApproximation,
} from '@nymproject/types';
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import Big from 'big.js';
import {
  EnumNodeType,
  isGateway,
  isMixnode,
  TBondGatewayArgs,
  TBondGatewaySignatureArgs,
  TBondMixNodeArgs,
  TBondMixnodeSignatureArgs,
  TUpdateBondArgs,
  TNodeDescription,
} from '@src/types';
import { Console } from '@src/utils/console';
import {
  bondGateway as bondGatewayRequest,
  bondMixNode as bondMixNodeRequest,
  claimOperatorReward,
  getGatewayBondDetails,
  getMixnodeBondDetails,
  unbondGateway as unbondGatewayRequest,
  unbondMixNode as unbondMixnodeRequest,
  vestingBondGateway,
  vestingBondMixNode,
  vestingUnbondGateway,
  vestingUnbondMixnode,
  updateMixnodeCostParams as updateMixnodeCostParamsRequest,
  vestingUpdateMixnodeCostParams as updateMixnodeVestingCostParamsRequest,
  getNodeDescription as getNodeDescriptionRequest,
  getMixnodeStatus,
  getPendingOperatorRewards,
  getMixnodeStakeSaturation,
  vestingClaimOperatorReward,
  getInclusionProbability,
  getMixnodeAvgUptime,
  getMixnodeRewardEstimation,
  getGatewayReport,
  getMixnodeUptime,
  vestingGenerateMixnodeMsgPayload as vestingGenerateMixnodeMsgPayloadReq,
  generateMixnodeMsgPayload as generateMixnodeMsgPayloadReq,
  vestingGenerateGatewayMsgPayload as vestingGenerateGatewayMsgPayloadReq,
  generateGatewayMsgPayload as generateGatewayMsgPayloadReq,
  updateBond as updateBondReq,
  vestingUpdateBond as vestingUpdateBondReq,
} from '../requests';
import { useCheckOwnership } from '../hooks/useCheckOwnership';
import { AppContext } from './main';
import {
  fireRequests,
  TauriReq,
  attachDefaultOperatingCost,
  decCoinToDisplay,
  toPercentFloatString,
  toPercentIntegerString,
  unymToNym,
} from '../utils';

export type TBondedMixnode = {
  name?: string;
  mixId: number;
  identityKey: string;
  stake: DecCoin;
  bond: DecCoin;
  stakeSaturation: string;
  uncappedStakeSaturation?: number;
  profitMargin: string;
  operatorRewards?: DecCoin;
  delegators: number;
  status: MixnodeStatus;
  proxy?: string;
  operatorCost: DecCoin;
  host: string;
  estimatedRewards?: DecCoin;
  activeSetProbability?: SelectionChance;
  standbySetProbability?: SelectionChance;
  routingScore: number;
  httpApiPort: number;
  mixPort: number;
  verlocPort: number;
  version: string;
  isUnbonding: boolean;
  uptime: number;
};

export interface TBondedGateway {
  name?: string;
  id: number;
  identityKey: string;
  ip: string;
  bond: DecCoin;
  location?: string;
  proxy?: string;
  host: string;
  httpApiPort: number;
  mixPort: number;
  verlocPort: number;
  version: string;
  routingScore?: {
    current: number;
    average: number;
  };
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
  updateBondAmount: (data: TUpdateBondArgs, tokenPool: TokenPool) => Promise<TransactionExecuteResult | undefined>;
  redeemRewards: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  updateMixnode: (pm: string, fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  generateMixnodeMsgPayload: (data: TBondMixnodeSignatureArgs) => Promise<string | undefined>;
  generateGatewayMsgPayload: (data: TBondGatewaySignatureArgs) => Promise<string | undefined>;
  isVestingAccount: boolean;
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
  updateBondAmount: async () => {
    throw new Error('Not implemented');
  },
  redeemRewards: async () => {
    throw new Error('Not implemented');
  },
  updateMixnode: async () => {
    throw new Error('Not implemented');
  },
  generateMixnodeMsgPayload: async () => {
    throw new Error('Not implemented');
  },
  generateGatewayMsgPayload: async () => {
    throw new Error('Not implemented');
  },
  isVestingAccount: false,
});

export const BondingContextProvider: FCWithChildren = ({ children }): JSX.Element => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [bondedNode, setBondedNode] = useState<TBondedMixnode | TBondedGateway>();
  const [isVestingAccount, setIsVestingAccount] = useState(false);

  const { userBalance, clientDetails } = useContext(AppContext);
  const { ownership, isLoading: isOwnershipLoading } = useCheckOwnership();

  useEffect(() => {
    userBalance.fetchBalance();
  }, [clientDetails]);

  useEffect(() => {
    if (userBalance.originalVesting) {
      setIsVestingAccount(true);
    }
  }, [userBalance]);

  const resetState = () => {
    setError(undefined);
    setBondedNode(undefined);
  };

  /**
   * Fetch mixnode **optional** data.
   * ⚠ The underlying queries are allowed to fail.
   */
  const fetchMixnodeDetails = async (mixId: number, host: string, port: number) => {
    const details: {
      status: MixnodeStatus;
      stakeSaturation: string;
      estimatedRewards?: DecCoin;
      uptime: number;
      averageUptime?: number;
      setProbability?: InclusionProbabilityResponse;
      nodeDescription?: TNodeDescription | undefined;
      operatorRewards?: DecCoin;
      uncappedSaturation?: number;
    } = {
      status: 'not_found',
      stakeSaturation: '0',
      uptime: 0,
    };

    const statusReq: TauriReq<typeof getMixnodeStatus> = {
      name: 'getMixnodeStatus',
      request: () => getMixnodeStatus(mixId),
      onFulfilled: (value) => {
        details.status = value.status;
      },
    };

    const uptimeReq: TauriReq<typeof getMixnodeUptime> = {
      name: 'getMixnodeUptime',
      request: () => getMixnodeUptime(mixId),
      onFulfilled: (value) => {
        details.uptime = value;
      },
    };

    const stakeSaturationReq: TauriReq<typeof getMixnodeStakeSaturation> = {
      name: 'getMixnodeStakeSaturation',
      request: () => getMixnodeStakeSaturation(mixId),
      onFulfilled: (value) => {
        details.stakeSaturation = decimalToPercentage(value.saturation);
        const rawUncappedSaturation = decimalToFloatApproximation(value.uncapped_saturation);
        if (rawUncappedSaturation && rawUncappedSaturation > 1) {
          details.uncappedSaturation = Math.round(rawUncappedSaturation * 100);
        }
      },
    };

    const rewardReq: TauriReq<typeof getMixnodeRewardEstimation> = {
      name: 'getMixnodeRewardEstimation',
      request: () => getMixnodeRewardEstimation(mixId),
      onFulfilled: (value) => {
        const estimatedRewards = unymToNym(value.estimation.total_node_reward);
        if (estimatedRewards) {
          details.estimatedRewards = {
            amount: estimatedRewards,
            denom: 'nym',
          };
        }
      },
    };

    const inclusionReq: TauriReq<typeof getInclusionProbability> = {
      name: 'getInclusionProbability',
      request: () => getInclusionProbability(mixId),
      onFulfilled: (value) => {
        details.setProbability = value;
      },
    };

    const avgUptimeReq: TauriReq<typeof getMixnodeAvgUptime> = {
      name: 'getMixnodeAvgUptime',
      request: () => getMixnodeAvgUptime(),
      onFulfilled: (value) => {
        details.averageUptime = value as number | undefined;
      },
    };

    const nodeDescReq: TauriReq<typeof getNodeDescriptionRequest> = {
      name: 'getNodeDescription',
      request: () => getNodeDescriptionRequest(host, port),
      onFulfilled: (value) => {
        details.nodeDescription = value;
      },
    };

    const operatorRewardsReq: TauriReq<typeof getPendingOperatorRewards> = {
      name: 'getPendingOperatorRewards',
      request: () => getPendingOperatorRewards(clientDetails?.client_address || ''),
      onFulfilled: (value) => {
        details.operatorRewards = decCoinToDisplay(value);
      },
    };

    await fireRequests([
      statusReq,
      uptimeReq,
      stakeSaturationReq,
      rewardReq,
      inclusionReq,
      avgUptimeReq,
      nodeDescReq,
      operatorRewardsReq,
    ]);

    return details;
  };

  /**
   * Fetch gateway **optional** data.
   * ⚠ The underlying queries are allowed to fail.
   */
  const fetchGatewayDetails = async (identityKey: string, host: string, port: number) => {
    const details: {
      routingScore?: { current: number; average: number } | undefined;
      nodeDescription?: TNodeDescription | undefined;
    } = {};

    const reportReq: TauriReq<typeof getGatewayReport> = {
      name: 'getGatewayReport',
      request: () => getGatewayReport(identityKey),
      onFulfilled: (value) => {
        details.routingScore = { current: value.most_recent, average: value.last_day };
      },
    };

    const nodeDescReq: TauriReq<typeof getNodeDescriptionRequest> = {
      name: 'getNodeDescription',
      request: () => getNodeDescriptionRequest(host, port),
      onFulfilled: (value) => {
        details.nodeDescription = value;
      },
    };

    await fireRequests([reportReq, nodeDescReq]);

    return details;
  };

  const calculateStake = (pledge: string, delegations: string) => {
    let stake;
    try {
      stake = unymToNym(Big(pledge).plus(delegations));
    } catch (e: any) {
      Console.warn(`not a valid decimal number: ${e}`);
    }
    return stake;
  };

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(undefined);

    if (ownership.hasOwnership && ownership.nodeType === EnumNodeType.mixnode && clientDetails) {
      try {
        const data = await getMixnodeBondDetails();
        if (data) {
          const {
            bond_information,
            rewarding_details,
            bond_information: { mix_id },
          } = data;

          const {
            status,
            stakeSaturation,
            uncappedSaturation: uncappedStakeSaturation,
            estimatedRewards,
            uptime,
            operatorRewards,
            averageUptime,
            nodeDescription,
            setProbability,
          } = await fetchMixnodeDetails(
            mix_id,
            bond_information.mix_node.host,
            bond_information.mix_node.http_api_port,
          );

          setBondedNode({
            id: data.bond_information.mix_id,
            name: nodeDescription?.name,
            mixId: mix_id,
            identityKey: bond_information.mix_node.identity_key,
            stake: {
              amount: calculateStake(rewarding_details.operator, rewarding_details.delegates),
              denom: bond_information.original_pledge.denom,
            },
            bond: decCoinToDisplay(bond_information.original_pledge),
            profitMargin: toPercentIntegerString(rewarding_details.cost_params.profit_margin_percent),
            delegators: rewarding_details.unique_delegations,
            proxy: bond_information.proxy,
            operatorRewards,
            uptime,
            status,
            stakeSaturation,
            uncappedStakeSaturation,
            operatorCost: decCoinToDisplay(rewarding_details.cost_params.interval_operating_cost),
            host: bond_information.mix_node.host.replace(/\s/g, ''),
            routingScore: averageUptime,
            activeSetProbability: setProbability?.in_active,
            standbySetProbability: setProbability?.in_reserve,
            estimatedRewards,
            httpApiPort: bond_information.mix_node.http_api_port,
            mixPort: bond_information.mix_node.mix_port,
            verlocPort: bond_information.mix_node.verloc_port,
            version: bond_information.mix_node.version,
            isUnbonding: bond_information.is_unbonding,
          } as TBondedMixnode);
        }
      } catch (e: any) {
        Console.warn(e);
        setError(`While fetching current bond state, an error occurred: ${e}`);
      }
    }

    if (ownership.hasOwnership && ownership.nodeType === EnumNodeType.gateway) {
      try {
        const data = await getGatewayBondDetails();
        if (data) {
          const { gateway, proxy } = data;
          const { nodeDescription, routingScore } = await fetchGatewayDetails(
            gateway.identity_key,
            data.gateway.host,
            data.gateway.clients_port,
          );
          setBondedNode({
            name: nodeDescription?.name,
            identityKey: gateway.identity_key,
            mixPort: gateway.mix_port,
            httpApiPort: gateway.clients_port,
            host: gateway.host,
            ip: gateway.host,
            location: gateway.location,
            bond: decCoinToDisplay(data.pledge_amount),
            proxy,
            routingScore,
            version: gateway.version,
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

  const updateBondAmount = async (data: TUpdateBondArgs, tokenPool: TokenPool) => {
    let tx: TransactionExecuteResult | undefined;
    setIsLoading(true);
    try {
      if (tokenPool === 'balance') {
        tx = await updateBondReq(data);
        await userBalance.fetchBalance();
      }
      if (tokenPool === 'locked') {
        tx = await vestingUpdateBondReq(data);
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

  const generateMixnodeMsgPayload = async (data: TBondMixnodeSignatureArgs) => {
    let message;
    setIsLoading(true);
    try {
      if (data.tokenPool === 'locked') {
        message = await vestingGenerateMixnodeMsgPayloadReq(data);
      } else {
        message = await generateMixnodeMsgPayloadReq(data);
      }
    } catch (e) {
      Console.warn(e);
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return message;
  };

  const generateGatewayMsgPayload = async (data: TBondGatewaySignatureArgs) => {
    let message;
    setIsLoading(true);
    try {
      if (data.tokenPool === 'locked') {
        message = await vestingGenerateGatewayMsgPayloadReq(data);
      } else {
        message = await generateGatewayMsgPayloadReq(data);
      }
    } catch (e) {
      Console.warn(e);
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return message;
  };

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
      updateBondAmount,
      generateMixnodeMsgPayload,
      generateGatewayMsgPayload,
      isVestingAccount,
    }),
    [isLoading, isOwnershipLoading, error, bondedNode, isVestingAccount],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
