import * as React from 'react';
import { contracts } from '@nymproject/contract-clients';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { Coin, GasPrice } from '@cosmjs/stargate';
import { settings } from '../client';

const signerAccount = async (mnemonic: string) => {
  // create a wallet to sign transactions with the mnemonic
  const signer = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: 'n',
  });

  return signer;
};

const fetchSignerCosmosWasmClient = async (mnemonic: string) => {
  const signer = await signerAccount(mnemonic);

  // create a signing client we don't need to set the gas price conversion for queries
  const cosmWasmClient = await SigningCosmWasmClient.connectWithSigner(settings.url, signer, {
    gasPrice: GasPrice.fromString('0.025unym'),
  });

  return cosmWasmClient;
};

const fetchSignerClient = async (mnemonic) => {
  const signer = await signerAccount(mnemonic);

  // create a signing client we don't need to set the gas price conversion for queries
  // if you want to connect without signer you'd write ".connect" and "url" as param
  const cosmWasmClient = await SigningCosmWasmClient.connectWithSigner(settings.url, signer, {
    gasPrice: GasPrice.fromString('0.025unym'),
  });

  /** create a mixnet contract client
   * @param cosmWasmClient the client to use for signing and querying
   * @param settings.address the bech32 address prefix (human readable part)
   * @param settings.mixnetContractAddress the bech32 address prefix (human readable part)
   * @returns the client in MixnetClient form
   */

  const mixnetClient = new contracts.Mixnet.MixnetClient(
    cosmWasmClient,
    settings.address, // sender (that account of the signer)
    settings.mixnetContractAddress, // contract address (different on mainnet, QA, etc)
  );

  return mixnetClient;
};

interface ApiState<RESPONSE> {
  isLoading: boolean;
  data?: RESPONSE;
  error?: Error;
}

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
}

export const WalletContext = React.createContext<WalletState>({
  accountLoading: false,
  account: '',
  clientsAreLoading: false,
  balanceLoading: false,
  sendingTokensLoading: false,
  log: [],
});

export const useWalletContext = (): React.ContextType<typeof WalletContext> =>
  React.useContext<WalletState>(WalletContext);

let cosmWasmSignerClient;
let nymWasmSignerClient;
let account;

export const WalletContextProvider = ({ children }: { children: JSX.Element }) => {
  const [accountLoading, setAccountLoading] = React.useState<boolean>(false);
  const [delegations, setDelegations] = React.useState<any>();
  const [clientsAreLoading, setClientsAreLoading] = React.useState<boolean>(false);
  const [balance, setBalance] = React.useState<Coin>(null);
  const [balanceLoading, setBalanceLoading] = React.useState<boolean>(false);
  const [sendingTokensLoading, setSendingTokensLoading] = React.useState<boolean>(false);
  const [log, setLog] = React.useState<React.ReactNode[]>([]);
  const [unDelegateAllLoading, setUnDelegateAllLoading] = React.useState<boolean>(false);

  const Reset = () => {
    setAccountLoading(false);
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

  const connect = React.useCallback(async (mnemonic: string) => {
    getSignerAccount(mnemonic);
    getClients(mnemonic);
  }, []);

  const getBalance = React.useCallback(async () => {
    setBalanceLoading(true);
    try {
      const newBalance = await cosmWasmSignerClient?.getBalance(account, 'unym');
      setBalance(newBalance);
    } catch (error) {
      console.error(error);
    }
    setBalanceLoading(false);
  }, [account, cosmWasmSignerClient]);

  const getDelegations = React.useCallback(async () => {
    const delegations = await nymWasmSignerClient.getDelegatorDelegations({
      delegator: settings.address,
    });
    console.log('delegations', delegations);
    setDelegations(delegations);
  }, [nymWasmSignerClient]);

  const sendTokens = React.useCallback(
    async (recipientAddress: string, tokensToSend: string) => {
      const memo = 'test sending tokens';
      setSendingTokensLoading(true);
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
    },
    [account, cosmWasmSignerClient],
  );

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

  React.useEffect(() => {
    return () => {
      Reset();
    };
  }, []);

  React.useEffect(() => {
    if (cosmWasmSignerClient) {
      getBalance();
    }
  }, [cosmWasmSignerClient]);

  React.useEffect(() => {
    if (nymWasmSignerClient) {
      getDelegations();
    }
  }, [nymWasmSignerClient]);

  const state = React.useMemo<WalletState>(
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
    ],
  );

  return <WalletContext.Provider value={state}>{children}</WalletContext.Provider>;
};
