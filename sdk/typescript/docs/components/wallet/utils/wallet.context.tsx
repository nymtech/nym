import React, { createContext, useContext, useState, useCallback, useEffect, useMemo } from 'react';
import { Coin } from '@cosmjs/stargate';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { settings } from '../../client';
import { signerAccount, fetchSignerCosmosWasmClient, fetchSignerClient } from './wallet.methods';

/**
 * This context provides the state for wallet.
 */

interface WalletState {
  accountLoading: boolean;
  account: string;
  clientsAreLoading: boolean;
  connect?: (mnemonic: string) => void;
  balance?: Coin;
  balanceLoading: boolean;
  setRecipientAddress?: (value: string) => void;
  setTokensToSend?: (value: string) => void;
  sendingTokensLoading: boolean;
  log?: { type: 'delegate' | 'sendTokens'; node: React.ReactNode[] };
  sendTokens?: (recipientAddress: string, tokensToSend: string) => void;
  delegations?: any;
  doDelegate?: (mixId: string, amount: string) => void;
  delegationLoader?: boolean;
  unDelegateAll?: () => void;
  unDelegateAllLoading?: boolean;
}

export const WalletContext = createContext<WalletState>({
  accountLoading: false,
  account: '',
  clientsAreLoading: false,
  balanceLoading: false,
  sendingTokensLoading: false,
});

export const useWalletContext = (): React.ContextType<typeof WalletContext> => useContext<WalletState>(WalletContext);

export const WalletContextProvider = ({ children }: { children: JSX.Element }) => {
  const [cosmWasmSignerClient, setCosmWasmSignerClient] = useState<SigningCosmWasmClient>(null);
  const [nymWasmSignerClient, setNymWasmSignerClient] = useState<any>(null);
  const [account, setAccount] = useState<string>('');
  const [accountLoading, setAccountLoading] = useState<boolean>(false);
  const [delegations, setDelegations] = useState<{ delegations: any[]; start_next_after: any }>();
  const [clientsAreLoading, setClientsAreLoading] = useState<boolean>(false);
  const [balance, setBalance] = useState<Coin>(null);
  const [balanceLoading, setBalanceLoading] = useState<boolean>(false);
  const [sendingTokensLoading, setSendingTokensLoading] = useState<boolean>(false);
  const [log, setLog] = useState<{ type: 'delegate' | 'sendTokens'; node: React.ReactNode[] }>();
  const [delegationLoader, setDelegationLoader] = useState<boolean>(false);
  const [unDelegateAllLoading, setUnDelegateAllLoading] = useState<boolean>(false);

  const Reset = () => {
    setAccountLoading(false);
    setDelegations(null);
    setClientsAreLoading(false);
    setBalance(null);
    setBalanceLoading(false);
    setSendingTokensLoading(false);
  };

  const getSignerAccount = async (mnemonic: string) => {
    setAccountLoading(true);
    try {
      const signer = await signerAccount(mnemonic);
      const accounts = await signer.getAccounts();
      if (accounts[0]) {
        setAccount(accounts[0].address);
      }
    } catch (error) {
      console.error(error);
    }
    setAccountLoading(false);
  };

  const getClients = async (mnemonic: string) => {
    setClientsAreLoading(true);
    try {
      setCosmWasmSignerClient(await fetchSignerCosmosWasmClient(mnemonic));
      setNymWasmSignerClient(await fetchSignerClient(mnemonic));
    } catch (error) {
      console.error(error);
    }
    setClientsAreLoading(false);
  };

  const connect = async (mnemonic: string) => {
    getSignerAccount(mnemonic);
    getClients(mnemonic);
  };

  const getBalance = useCallback(async () => {
    setBalanceLoading(true);
    try {
      const newBalance = await cosmWasmSignerClient?.getBalance(account, 'unym');
      setBalance(newBalance);
    } catch (error) {
      console.error(error);
    }
    setBalanceLoading(false);
  }, [account, cosmWasmSignerClient]);

  const getDelegations = useCallback(async () => {
    const delegationsReceived = await nymWasmSignerClient.getDelegatorDelegations({
      delegator: settings.address,
    });
    setDelegations(delegationsReceived);
  }, [nymWasmSignerClient]);

  const sendTokens = async (recipientAddress: string, tokensToSend: string) => {
    const memo: string = 'test sending tokens';
    setSendingTokensLoading(true);
    try {
      const res = await cosmWasmSignerClient.sendTokens(
        account,
        recipientAddress,
        [{ amount: tokensToSend, denom: 'unym' }],
        'auto',
        memo,
      );
      setLog({
        type: 'sendTokens',
        node: [
          <div key={JSON.stringify(res, null, 2)}>
            <code style={{ marginRight: '2rem' }}>{new Date().toLocaleTimeString()}</code>
            <pre>{JSON.stringify(res, null, 2)}</pre>
          </div>,
        ],
      });
    } catch (error) {
      console.error(error);
    }
    setSendingTokensLoading(false);
  };

  const doDelegate = async (mixId: string, amount: string) => {
    setDelegationLoader(true);
    const memo: string = 'test delegation';
    const coinAmount: Coin = { amount, denom: 'unym' };
    try {
      const res = await nymWasmSignerClient.delegateToMixnode({ mixId: parseInt(mixId, 10) }, 'auto', memo, [
        coinAmount,
      ]);
      setLog({
        type: 'delegate',
        node: [
          <div key={JSON.stringify(res, null, 2)}>
            <code style={{ marginRight: '2rem' }}>{new Date().toLocaleTimeString()}</code>
            <pre>{JSON.stringify(res, null, 2)}</pre>
          </div>,
        ],
      });
    } catch (error) {
      console.error(error);
    }
    setDelegationLoader(false);
  };

  const unDelegateAll = async () => {
    setUnDelegateAllLoading(true);
    try {
      const logs: React.ReactNode[] = [];
      // eslint-disable-next-line no-restricted-syntax
      for (const delegation of delegations.delegations) {
        // eslint-disable-next-line no-await-in-loop
        const res = await nymWasmSignerClient.undelegateFromMixnode({ mixId: delegation.mix_id }, 'auto');
        setUnDelegateAllLoading(false);
        logs.push(
          <div key={JSON.stringify(res, null, 2)}>
            <code style={{ marginRight: '2rem' }}>{new Date().toLocaleTimeString()}</code>
            <pre>{JSON.stringify(res, null, 2)}</pre>
          </div>,
        );
      }
      setLog({
        type: 'delegate',
        node: logs,
      });
    } catch (error) {
      console.error(error);
      setUnDelegateAllLoading(false);
    }
  };

  // const withdrawRewards = async () => {
  //   const validatorAdress = '';
  //   const memo = 'test withdraw rewards';
  //   setWithdrawLoading(true);
  //   try {
  //     const res = await cosmWasmSignerClient.withdrawRewards(account, validatorAdress, 'auto', memo);
  //     setLog({
  //       type: 'delegate',
  //       node: [
  //         <div key={JSON.stringify(res, null, 2)}>
  //           <code style={{ marginRight: '2rem' }}>{new Date().toLocaleTimeString()}</code>
  //           <pre>{JSON.stringify(res, null, 2)}</pre>
  //         </div>,
  //       ],
  //     });
  //   } catch (error) {
  //     console.error(error);
  //   }
  //   setWithdrawLoading(false);
  // };

  useEffect(
    () => () => {
      Reset();
    },
    [],
  );

  useEffect(() => {
    if (cosmWasmSignerClient) {
      getBalance();
    }
  }, [cosmWasmSignerClient]);

  useEffect(() => {
    if (nymWasmSignerClient) {
      getDelegations();
    }
  }, [nymWasmSignerClient]);

  const state = useMemo<WalletState>(
    () => ({
      accountLoading,
      account,
      clientsAreLoading,
      connect,
      balance,
      balanceLoading,
      sendingTokensLoading,
      log,
      sendTokens,
      delegations,
      doDelegate,
      delegationLoader,
      unDelegateAll,
      unDelegateAllLoading,
    }),
    [
      accountLoading,
      account,
      clientsAreLoading,
      connect,
      balance,
      balanceLoading,
      sendingTokensLoading,
      log,
      sendTokens,
      delegations,
      doDelegate,
      delegationLoader,
      unDelegateAll,
      unDelegateAllLoading,
    ],
  );

  return <WalletContext.Provider value={state}>{children}</WalletContext.Provider>;
};
