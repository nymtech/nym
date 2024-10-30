/* eslint-disable @typescript-eslint/naming-convention */
import React, { createContext, useContext, useEffect, useMemo, useState } from 'react';
import { FeeDetails, TransactionExecuteResult } from '@nymproject/types';
import { isGateway, isMixnode, TUpdateBondArgs, isNymNode, TNymNodeSignatureArgs, TBondNymNodeArgs } from 'src/types';
import { Console } from 'src/utils/console';
import useGetNodeDetails from 'src/hooks/useGetNodeDetails';
import { TBondedNymNode } from 'src/requests/nymNodeDetails';
import { TBondedMixnode } from 'src/requests/mixnodeDetails';
import { TBondedGateway } from 'src/requests/gatewayDetails';
import { toPercentFloatString } from 'src/utils';
import { AppContext } from './main';
import {
  claimOperatorReward,
  unbondGateway as unbondGatewayRequest,
  unbondMixNode as unbondMixnodeRequest,
  unbondNymNode as unbondNymNodeRequest,
  vestingClaimOperatorReward,
  generateNymNodeMsgPayload as generateNymNodeMsgPayloadReq,
  updateBond as updateBondReq,
  migrateVestedMixnode as tauriMigrateVestedMixnode,
  migrateLegacyMixnode as migrateLegacyMixnodeReq,
  migrateLegacyGateway as migrateLegacyGatewayReq,
  bondNymNode,
} from '../requests';

export type TBondedNode = TBondedMixnode | TBondedGateway | TBondedNymNode;

export type TBondingContext = {
  isLoading: boolean;
  error?: string;
  bondedNode?: TBondedNode | null;
  isVestingAccount: boolean;
  refresh: () => void;
  unbond: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  bond: (args: TBondNymNodeArgs) => Promise<TransactionExecuteResult | undefined>;
  updateBondAmount: (data: TUpdateBondArgs) => Promise<TransactionExecuteResult | undefined>;
  redeemRewards: (fee?: FeeDetails) => Promise<TransactionExecuteResult | undefined>;
  generateNymNodeMsgPayload: (data: TNymNodeSignatureArgs) => Promise<string | undefined>;
  migrateVestedMixnode: () => Promise<TransactionExecuteResult | undefined>;
  migrateLegacyNode: () => Promise<TransactionExecuteResult | undefined>;
};

export const BondingContext = createContext<TBondingContext>({
  isLoading: true,
  refresh: async () => undefined,
  bond: async () => {
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
  generateNymNodeMsgPayload: async () => {
    throw new Error('Not implemented');
  },
  migrateVestedMixnode: async () => {
    throw new Error('Not implemented');
  },
  migrateLegacyNode: async () => {
    throw new Error('Not implemented');
  },
  isVestingAccount: false,
});

export const BondingContextProvider: FCWithChildren = ({ children }): JSX.Element => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string>();

  const [isVestingAccount, setIsVestingAccount] = useState(false);

  const { userBalance, clientDetails, network } = useContext(AppContext);

  const {
    bondedNode,
    isLoading: isBondedNodeLoading,
    getNodeDetails,
  } = useGetNodeDetails(clientDetails?.client_address, network);

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

  const bond = async (data: TBondNymNodeArgs) => {
    let tx;
    setIsLoading(true);

    try {
      tx = await bondNymNode({
        ...data,
        costParams: {
          ...data.costParams,
          profit_margin_percent: toPercentFloatString(data.costParams.profit_margin_percent),
        },
      });
      if (clientDetails?.client_address) {
        await getNodeDetails(clientDetails?.client_address);
      }
    } catch (e) {
      Console.warn(e);
      setError(`an error occurred: ${e as string}`);
    } finally {
      setIsLoading(false);
    }
    return tx;
  };

  const unbond = async (fee?: FeeDetails) => {
    let tx;
    setIsLoading(true);
    try {
      if (bondedNode && isNymNode(bondedNode)) tx = await unbondNymNodeRequest(fee?.fee);
      if (bondedNode && isMixnode(bondedNode) && !bondedNode.proxy) tx = await unbondMixnodeRequest(fee?.fee);
      if (bondedNode && isGateway(bondedNode) && !bondedNode.proxy) tx = await unbondGatewayRequest(fee?.fee);
    } catch (e) {
      Console.warn(e);
      setError(`an error occurred: ${e as string}`);
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

  const updateBondAmount = async (data: TUpdateBondArgs) => {
    let tx: TransactionExecuteResult | undefined;
    setIsLoading(true);
    try {
      tx = await updateBondReq(data);
      await userBalance.fetchBalance();

      return tx;
    } catch (e: any) {
      Console.warn(e);
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return undefined;
  };

  const generateNymNodeMsgPayload = async (data: TNymNodeSignatureArgs) => {
    setIsLoading(true);

    try {
      const message = await generateNymNodeMsgPayloadReq({
        nymnode: data.nymnode,
        pledge: data.pledge,
        costParams: {
          ...data.costParams,
          profit_margin_percent: toPercentFloatString(data.costParams.profit_margin_percent),
        },
      });
      return message;
    } catch (e) {
      Console.warn(e);
      setError(`an error occurred: ${e}`);
    } finally {
      setIsLoading(false);
    }
    return undefined;
  };

  const migrateVestedMixnode = async () => {
    setIsLoading(true);
    const tx = await tauriMigrateVestedMixnode();
    setIsLoading(false);
    return tx;
  };

  const migrateLegacyNode = async () => {
    setIsLoading(true);
    let tx: TransactionExecuteResult | undefined;

    if (bondedNode && isMixnode(bondedNode)) {
      tx = await migrateLegacyMixnodeReq();
    }
    if (bondedNode && isGateway(bondedNode)) {
      tx = await migrateLegacyGatewayReq();
    }

    setIsLoading(false);
    return tx;
  };

  const memoizedValue = useMemo(
    () => ({
      isLoading: isLoading || isBondedNodeLoading,
      error,
      bondedNode,
      bond,
      unbond,
      refresh,
      redeemRewards,
      updateBondAmount,
      generateNymNodeMsgPayload,
      migrateVestedMixnode,
      migrateLegacyNode,
      isVestingAccount,
    }),
    [isLoading, error, bondedNode, isVestingAccount, isBondedNodeLoading],
  );

  return <BondingContext.Provider value={memoizedValue}>{children}</BondingContext.Provider>;
};

export const useBondingContext = () => useContext<TBondingContext>(BondingContext);
