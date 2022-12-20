/* eslint-disable @typescript-eslint/naming-convention */
import {
  FeeDetails,
  DecCoin,
  MixnodeStatus,
  TransactionExecuteResult,
  decimalToPercentage,
  SelectionChance,
} from '@nymproject/types';
import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import Big from 'big.js';
import {
  EnumNodeType,
  isGateway,
  isMixnode,
  TBondGatewayArgs,
  TBondGatewaySignatureArgs,
  TBondMixNodeArgs,
  TBondMixnodeSignatureArgs,
  TBondMoreArgs,
} from 'src/types';
import { Console } from 'src/utils/console';
import {
  bondGateway as bondGatewayRequest,
  bondMixNode as bondMixNodeRequest,
  claimOperatorReward,
  getGatewayBondDetails,
  getMixnodeBondDetails,
  unbondGateway as unbondGatewayRequest,
  unbondMixNode as unbondMixnodeRequest,
  bondMore as bondMoreRequest,
  vestingBondMore,
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
} from '../requests';
import { useCheckOwnership } from '../hooks/useCheckOwnership';
import { AppContext } from './main';
import {
  attachDefaultOperatingCost,
  decCoinToDisplay,
  toDisplay,
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
  bondMore: (data: TBondMoreArgs, tokenPool: TokenPool) => Promise<TransactionExecuteResult | undefined>;
  redeemRewards: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  updateMixnode: (pm: string, fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  checkOwnership: () => Promise<void>;
  generateMixnodeMsgPayload: (data: TBondMixnodeSignatureArgs) => Promise<string | undefined>;
  generateGatewayMsgPayload: (data: TBondGatewaySignatureArgs) => Promise<string | undefined>;
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
  bondMore: async () => {
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
  generateMixnodeMsgPayload: async () => {
    throw new Error('Not implemented');
  },
  generateGatewayMsgPayload: async () => {
    throw new Error('Not implemented');
  },
});

export const BondingContextProvider: FCWithChildren = ({ children }): JSX.Element => {
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
    const additionalDetails: {
      status: MixnodeStatus;
      stakeSaturation: string;
      estimatedRewards?: DecCoin;
      uptime: number;
    } = {
      status: 'not_found',
      stakeSaturation: '0',
      uptime: 0,
    };

    try {
      const statusResponse = await getMixnodeStatus(mixId);
      const uptime = await getMixnodeUptime(mixId);
      additionalDetails.status = statusResponse.status;
      additionalDetails.uptime = uptime;
    } catch (e) {
      Console.log('getMixnodeStatus fails', e);
    }

    try {
      const stakeSaturationResponse = await getMixnodeStakeSaturation(mixId);
      additionalDetails.stakeSaturation = decimalToPercentage(stakeSaturationResponse.saturation);
    } catch (e) {
      Console.log('getMixnodeStakeSaturation fails', e);
    }
    try {
      const rewardEstimation = await getMixnodeRewardEstimation(mixId);
      const estimatedRewards = unymToNym(rewardEstimation.estimation.total_node_reward);
      if (estimatedRewards) {
        additionalDetails.estimatedRewards = {
          amount: estimatedRewards,
          denom: 'nym',
        };
      }
    } catch (e) {
      Console.log('getMixnodeRewardEstimation fails', e);
    }
    return additionalDetails;
  };

  const getNodeDescription = async (host: string, port: number) => {
    let result;
    try {
      result = await getNodeDescriptionRequest(host, port);
    } catch (e) {
      Console.log('getNodeDescriptionRequest fails', e);
    }
    return result;
  };

  const getSetProbabilities = async (mixId: number) => {
    let result;
    try {
      result = await getInclusionProbability(mixId);
    } catch (e: any) {
      Console.log('getInclusionProbability fails', e);
    }
    return result;
  };

  const getAvgUptime = async () => {
    let result;
    try {
      result = await getMixnodeAvgUptime();
    } catch (e: any) {
      Console.log('getMixnodeAvgUptime fails', e);
    }
    return result;
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

  const getGatewayReportDetails = async (identityKey: string) => {
    try {
      const report = await getGatewayReport(identityKey);
      return { current: report.most_recent, average: report.last_day };
    } catch (e) {
      Console.error(e);
      return undefined;
    }
  };

  const refresh = useCallback(async () => {
    setIsLoading(true);

    if (ownership.hasOwnership && clientDetails) {
      try {
        const data = await getMixnodeBondDetails();
        let operatorRewards;
        try {
          operatorRewards = await getPendingOperatorRewards(clientDetails?.client_address);
          const opRewards = toDisplay(operatorRewards.amount);
          if (opRewards) {
            operatorRewards.amount = opRewards;
          }
        } catch (e) {
          Console.warn(`get_operator_rewards request failed: ${e}`);
        }
        if (data) {
          const {
            bond_information,
            rewarding_details,
            bond_information: { mix_id },
          } = data;

          const { status, stakeSaturation, estimatedRewards, uptime } = await getAdditionalMixnodeDetails(mix_id);
          const setProbabilities = await getSetProbabilities(mix_id);
          const nodeDescription = await getNodeDescription(
            bond_information.mix_node.host,
            bond_information.mix_node.http_api_port,
          );
          const routingScore = await getAvgUptime();
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
            operatorCost: decCoinToDisplay(rewarding_details.cost_params.interval_operating_cost),
            host: bond_information.mix_node.host.replace(/\s/g, ''),
            routingScore,
            activeSetProbability: setProbabilities?.in_active,
            standbySetProbability: setProbabilities?.in_reserve,
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
          const nodeDescription = await getNodeDescription(data.gateway.host, data.gateway.clients_port);
          const routingScore = await getGatewayReportDetails(data.gateway.identity_key);
          setBondedNode({
            name: nodeDescription?.name,
            identityKey: gateway.identity_key,
            mixPort: gateway.mix_port,
            httpApiPort: gateway.clients_port,
            host: gateway.host,
            ip: gateway.host,
            location: gateway.location,
            bond: decCoinToDisplay(data.pledge_amount),
            proxy: proxy,
            routingScore,
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

  const bondMore = async (data: TBondMoreArgs, tokenPool: TokenPool) => {
    let tx: TransactionExecuteResult | undefined;
    setIsLoading(true);
    try {
      if (tokenPool === 'balance') {
        tx = await bondMoreRequest(data);
        await userBalance.fetchBalance();
      }
      if (tokenPool === 'locked') {
        tx = await vestingBondMore(data);
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
      if (isVesting) {
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
      if (isVesting) {
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
      bondMore,
      checkOwnership,
      generateMixnodeMsgPayload,
      generateGatewayMsgPayload,
    }),
    [isLoading, isOwnershipLoading, error, bondedNode, isVesting],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
