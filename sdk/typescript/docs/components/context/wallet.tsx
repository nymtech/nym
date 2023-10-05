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
  cosmWasmSigner?: { getAccounts: () => void };
  cosmWasmSignerClient?: {
    getBalance: (account: string, denom: string) => Coin;
    sendTokens: (account: string, recipientAddress: string, amount: [Coin], type: 'auto', memo: string) => void;
  };
  nymWasmSignerClient?: ApiState<any>;
  accountLoading: boolean;
  account: string;
  clientsAreLoading: boolean;
  setConnectWithMnemonic?: (value: string) => void;
  balance?: Coin;
  balanceLoading: boolean;
  setRecipientAddress?: (value: string) => void;
  setTokensToSend?: (value: string) => void;
  sendingTokensLoading: boolean;
  log: React.ReactNode[];
  doSendTokens?: () => void;
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

export const WalletContextProvider = ({ children }: { children: JSX.Element }) => {
  // wallet mnemonic
  const [connectWithMnemonic, setConnectWithMnemonic] = React.useState<string>();
  const [accountLoading, setAccountLoading] = React.useState<boolean>(false);
  const [account, setAccount] = React.useState<string>();
  const [clientsAreLoading, setClientsAreLoading] = React.useState<boolean>(false);
  const [cosmWasmSignerClient, setCosmWasmSignerClient] = React.useState<any>();
  const [nymWasmSignerClient, setNymWasmSignerClient] = React.useState<any>();
  const [balance, setBalance] = React.useState<Coin>();
  const [balanceLoading, setBalanceLoading] = React.useState<boolean>(false);
  const [recipientAddress, setRecipientAddress] = React.useState<string>('');
  const [tokensToSend, setTokensToSend] = React.useState<string>();
  const [sendingTokensLoading, setSendingTokensLoading] = React.useState<boolean>(false);
  const [log, setLog] = React.useState<React.ReactNode[]>([]);

  const getSignerAccount = async () => {
    setAccountLoading(true);
    try {
      const signer = await signerAccount(connectWithMnemonic);
      const accounts = await signer.getAccounts();
      if (accounts[0]) {
        setAccount(accounts[0].address);
      }
    } catch (error) {
      console.error(error);
    }
    setAccountLoading(false);
  };
  const getClients = async () => {
    setClientsAreLoading(true);
    try {
      console.log('setCosmWasmSignerClient');
      setCosmWasmSignerClient(await fetchSignerCosmosWasmClient(connectWithMnemonic));
      setNymWasmSignerClient(await fetchSignerClient(connectWithMnemonic));
    } catch (error) {
      console.error(error);
    }
    setClientsAreLoading(false);
  };

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

  // Sending tokens
  const doSendTokens = React.useCallback(async () => {
    const memo = 'test sending tokens';
    setSendingTokensLoading(true);
    try {
      console.log('cosmWasmSignerClient', cosmWasmSignerClient, account, recipientAddress);
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
  }, [account, cosmWasmSignerClient]);
  // End send tokens

  React.useEffect(() => {
    if (connectWithMnemonic) {
      // when the mnemonic changes, remove all previous data
      Promise.all([getSignerAccount(), getClients()]);
    }
  }, [connectWithMnemonic]);

  React.useEffect(() => {
    console.log('cosmWasmSignerClient', cosmWasmSignerClient);
  }, [cosmWasmSignerClient]);

  React.useEffect(() => {
console.log('tokensToSend', tokensToSend);

  },[tokensToSend])

  React.useEffect(() => {
    if (account && cosmWasmSignerClient) {
      if (!balance) {
        getBalance();
      }
    }
  }, [account, cosmWasmSignerClient, balance, getBalance]);

  const state = React.useMemo<WalletState>(
    () => ({
      accountLoading,
      account,
      clientsAreLoading,
      cosmWasmSignerClient,
      nymWasmSignerClient,
      setConnectWithMnemonic,
      balance,
      balanceLoading,
      setRecipientAddress,
      setTokensToSend,
      sendingTokensLoading,
      log,
      doSendTokens,
    }),
    [
      accountLoading,
      account,
      clientsAreLoading,
      cosmWasmSignerClient,
      nymWasmSignerClient,
      setConnectWithMnemonic,
      balance,
      balanceLoading,
      setRecipientAddress,
      setTokensToSend,
      sendingTokensLoading,
      log,
      doSendTokens,
    ],
  );

  return <WalletContext.Provider value={state}>{children}</WalletContext.Provider>;
};
