import { createContext, useContext, useState, useCallback, useEffect, useMemo } from 'react';
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
  log: React.ReactNode[];
  sendTokens?: (recipientAddress: string, tokensToSend: string) => void;
  delegations?: any;
  unDelegateAll?: () => void;
  unDelegateAllLoading?: boolean;
  doDelegate?: ({ mixId, amount }: { mixId: number; amount: number }) => void;
  delegationLoader?: boolean;
  withdrawLoading?: boolean;
  withdrawRewards?: () => void;
}

export const WalletContext = createContext<WalletState>({
  accountLoading: false,
  account: '',
  clientsAreLoading: false,
  balanceLoading: false,
  sendingTokensLoading: false,
  log: [],
});

export const useWalletContext = (): React.ContextType<typeof WalletContext> => useContext<WalletState>(WalletContext);

let cosmWasmSignerClient: SigningCosmWasmClient;
let nymWasmSignerClient: any;
let account: string;

export const WalletContextProvider = ({ children }: { children: JSX.Element }) => {
  const [accountLoading, setAccountLoading] = useState<boolean>(false);
  const [delegations, setDelegations] = useState<any>();
  const [clientsAreLoading, setClientsAreLoading] = useState<boolean>(false);
  const [balance, setBalance] = useState<Coin>(null);
  const [balanceLoading, setBalanceLoading] = useState<boolean>(false);
  const [sendingTokensLoading, setSendingTokensLoading] = useState<boolean>(false);
  const [log, setLog] = useState<React.ReactNode[]>([]);
  const [delegationLoader, setDelegationLoader] = useState<boolean>(false);
  const [unDelegateAllLoading, setUnDelegateAllLoading] = useState<boolean>(false);
  const [withdrawLoading, setWithdrawLoading] = useState<boolean>(false);

  const Reset = () => {
    setAccountLoading(false);
    setDelegations(null);
    setClientsAreLoading(false);
    setBalance(null);
    setBalanceLoading(false);
    setSendingTokensLoading(false);
    setLog([]);
  };

  const getSignerAccount = async (mnemonic: string) => {
    setAccountLoading(true);
    try {
      const signer = await signerAccount(mnemonic);
      const accounts = await signer.getAccounts();
      if (accounts[0]) {
        account = accounts[0].address;
      }
    } catch (error) {
      console.error(error);
    }
    setAccountLoading(false);
  };

  const getClients = async (mnemonic: string) => {
    setClientsAreLoading(true);
    try {
      cosmWasmSignerClient = await fetchSignerCosmosWasmClient(mnemonic);
      nymWasmSignerClient = await fetchSignerClient(mnemonic);
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
    const delegations = await nymWasmSignerClient.getDelegatorDelegations({
      delegator: settings.address,
    });
    console.log('delegations', delegations);
    setDelegations(delegations);
  }, [nymWasmSignerClient]);

  const sendTokens = async (recipientAddress: string, tokensToSend: string) => {
    const memo = 'test sending tokens';
    setSendingTokensLoading(true);
    console.log([{ amount: tokensToSend, denom: 'unym' }],)
    try {
      const res = await cosmWasmSignerClient.sendTokens(
        account,
        recipientAddress,
        [{ amount: tokensToSend, denom: 'unym' }],
        'auto',
        memo,
      );
      setLog((prev) => [
        ...prev,
        <div key={JSON.stringify(res, null, 2)}>
          <code style={{ marginRight: '2rem' }}>{new Date().toLocaleTimeString()}</code>
          <pre>{JSON.stringify(res, null, 2)}</pre>
        </div>,
      ]);
    } catch (error) {
      console.error(error);
    }
    setSendingTokensLoading(false);
  };

  const doDelegate = async ({ mixId, amount }: { mixId: number; amount: number }) => {
    setDelegationLoader(true);
    try {
      const res = await nymWasmSignerClient.delegateToMixnode({ mixId }, 'auto', undefined, [
        { amount: `${amount}`, denom: 'unym' },
      ]);
      setLog((prev) => [
        ...prev,
        <div key={JSON.stringify(res, null, 2)}>
          <code style={{ marginRight: '2rem' }}>{new Date().toLocaleTimeString()}</code>
          <pre>{JSON.stringify(res, null, 2)}</pre>
        </div>,
      ]);
    } catch (error) {
      console.error(error);
    }
    setDelegationLoader(false);
  };

  const withdrawRewards = async () => {
    const validatorAdress = '';
    const memo = 'test withdraw rewards';
    setWithdrawLoading(true);
    try {
      const res = await cosmWasmSignerClient.withdrawRewards(account, validatorAdress, 'auto', memo);
      setLog((prev) => [
        ...prev,
        <div key={JSON.stringify(res, null, 2)}>
          <code style={{ marginRight: '2rem' }}>{new Date().toLocaleTimeString()}</code>
          <pre>{JSON.stringify(res, null, 2)}</pre>
        </div>,
      ]);
    } catch (error) {
      console.error(error);
    }
    setWithdrawLoading(false);
  };

  const undelegateAll = async () => {
    if (!nymWasmSignerClient) {
      return;
    }
    setUnDelegateAllLoading(true);
    try {
      // eslint-disable-next-line no-restricted-syntax
      for (const delegation of delegations.delegations) {
        // eslint-disable-next-line no-await-in-loop
        await nymWasmSignerClient.undelegateFromMixnode({ mixId: delegation.mix_id }, 'auto');
      }
    } catch (error) {
      console.error(error);
    }
    setUnDelegateAllLoading(false);
  };

  useEffect(() => {
    return () => {
      Reset();
    };
  }, []);

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
      undelegateAll,
      unDelegateAllLoading,
      doDelegate,
      delegationLoader,
      withdrawRewards,
      withdrawLoading,
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
      undelegateAll,
      unDelegateAllLoading,
      doDelegate,
      delegationLoader,
      withdrawRewards,
      withdrawLoading,
    ],
  );

  return <WalletContext.Provider value={state}>{children}</WalletContext.Provider>;
};
