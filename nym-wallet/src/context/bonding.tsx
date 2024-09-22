/* eslint-disable @typescript-eslint/naming-convention */
import { FeeDetails, DecCoin, MixnodeStatus, TransactionExecuteResult, SelectionChance } from '@nymproject/types';
import { createContext, useContext, useEffect, useMemo, useState } from 'react';
import {
  isGateway,
  isMixnode,
  TBondGatewayArgs,
  TBondGatewaySignatureArgs,
  TBondMixNodeArgs,
  TBondMixnodeSignatureArgs,
  TUpdateBondArgs,
  isNymNode,
} from 'src/types';
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
  vestingClaimOperatorReward,
  vestingGenerateMixnodeMsgPayload as vestingGenerateMixnodeMsgPayloadReq,
  generateMixnodeMsgPayload as generateMixnodeMsgPayloadReq,
  vestingGenerateGatewayMsgPayload as vestingGenerateGatewayMsgPayloadReq,
  generateGatewayMsgPayload as generateGatewayMsgPayloadReq,
  updateBond as updateBondReq,
  vestingUpdateBond as vestingUpdateBondReq,
  migrateVestedMixnode as tauriMigrateVestedMixnode,
} from '../requests';
import { useCheckOwnership } from '../hooks/useCheckOwnership';
import { AppContext } from './main';
import { attachDefaultOperatingCost, toPercentFloatString } from '../utils';
import useGetNodeDetails from 'src/hooks/useGetNodeDetails';

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
  proxy?: string | null;
  operatorCost: DecCoin;
  host: string;
  estimatedRewards?: DecCoin;
  activeSetProbability?: SelectionChance;
  standbySetProbability?: SelectionChance;
  routingScore?: number;
  httpApiPort: number;
  mixPort: number;
  verlocPort: number;
  version: string;
  isUnbonding: boolean;
  uptime: number;
};

export interface TBondedGateway {
  name?: string;
  identityKey: string;
  ip: string;
  bond: DecCoin;
  location: string;
  proxy: string | null;
  host: string;
  httpApiPort: number;
  mixPort: number;
  version: string;
  routingScore?: {
    current: number;
    average: number;
  };
}

export type TBondedNymNode = {
  nodeId: number;
};

export type TBondedNode = TBondedMixnode | TBondedGateway | TBondedNymNode;

export type TokenPool = 'locked' | 'balance';

export type TBondingContext = {
  isLoading: boolean;
  error?: string;
  bondedNode?: TBondedNode | null;
  isVestingAccount: boolean;
  refresh: () => void;
  bondMixnode: (data: TBondMixNodeArgs, tokenPool: TokenPool) => Promise<TransactionExecuteResult | undefined>;
  bondGateway: (data: TBondGatewayArgs, tokenPool: TokenPool) => Promise<TransactionExecuteResult | undefined>;
  unbond: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  updateBondAmount: (data: TUpdateBondArgs, tokenPool: TokenPool) => Promise<TransactionExecuteResult | undefined>;
  redeemRewards: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  updateMixnode: (pm: string, fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  generateMixnodeMsgPayload: (data: TBondMixnodeSignatureArgs) => Promise<string | undefined>;
  generateGatewayMsgPayload: (data: TBondGatewaySignatureArgs) => Promise<string | undefined>;
  migrateVestedMixnode: () => Promise<TransactionExecuteResult | undefined>;
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
  migrateVestedMixnode: async () => {
    throw new Error('Not implemented');
  },
  isVestingAccount: false,
});

export const BondingContextProvider: FCWithChildren = ({ children }): JSX.Element => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();

  const [isVestingAccount, setIsVestingAccount] = useState(false);

  const { userBalance, clientDetails, network } = useContext(AppContext);

  const { bondedNode, isLoading: isBondedNodeLoading } = useGetNodeDetails(clientDetails?.client_address, network);

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
    setIsLoading(false);
  };

  const refresh = () => {
    resetState();
  };

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
      if (bondedNode && isMixnode(bondedNode) && bondedNode.proxy) {
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
      if (bondedNode && !isNymNode(bondedNode)) tx = await vestingClaimOperatorReward(fee?.fee);
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

  const migrateVestedMixnode = async () => {
    setIsLoading(true);
    const tx = await tauriMigrateVestedMixnode();
    setIsLoading(false);
    return tx;
  };

  const memoizedValue = useMemo(
    () => ({
      isLoading: isLoading || isBondedNodeLoading,
      error,
      bondedNode,
      bondMixnode,
      bondGateway,
      unbond,
      updateMixnode,
      refresh,
      redeemRewards,
      updateBondAmount,
      generateMixnodeMsgPayload,
      generateGatewayMsgPayload,
      migrateVestedMixnode,
      isVestingAccount,
    }),
    [isLoading, error, bondedNode, isVestingAccount, isBondedNodeLoading],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
