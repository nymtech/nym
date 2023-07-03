import React, { useEffect, useMemo, useState } from 'react';
import ValidatorClient from '@nymproject/nym-validator-client';
import { ExtensionStorage } from '@nymproject/extension-storage';
import { connectToValidator } from 'src/validator-client';
import { unymToNym } from 'src/utils/coin';
import { Currency, getTokenPrice } from 'src/utils/price';

type TAppContext = {
  client?: ValidatorClient;
  accounts: string[];
  balance?: string;
  fiatBalance?: number;
  denom: 'NYM';
  minorDenom: 'unym';
  currency: Currency;
  showSeedForAccount?: string;
  selectedAccount: string;
  storage?: ExtensionStorage;
  selectAccount: (accountName: string) => Promise<void>;
  setAccounts: (accounts: string[]) => void;
  setShowSeedForAccount: (accountName?: string) => void;
  handleUnlockWallet: (password: string) => void;
  getBalance: () => void;
};

type TBalanceInNYMs = string;

const DEFAULT_ACCOUNT_NAME = 'Default account';

const AppContext = React.createContext({} as TAppContext);

export const AppProvider = ({ children }: { children: React.ReactNode }) => {
  const [client, setClient] = useState<ValidatorClient>();
  const [selectedAccount, setSelected] = useState<string>(DEFAULT_ACCOUNT_NAME);
  const [balance, setBalance] = useState<TBalanceInNYMs>();
  const [fiatBalance, setFiatBalance] = useState<number>();
  const [accounts, setAccounts] = useState<string[]>([]);
  const [showSeedForAccount, setShowSeedForAccount] = useState<string>();
  const [storage, setStorage] = useState<ExtensionStorage>();

  const denom = 'NYM';
  const minorDenom = 'unym';
  const currency = 'gbp';

  const handleUnlockWallet = async (password: string) => {
    const store = await new ExtensionStorage(password);
    const mnemonic = await store.read_mnemonic(DEFAULT_ACCOUNT_NAME);
    const userAccounts = await store.get_all_mnemonic_keys();
    const clientFromMnemonic = await connectToValidator(mnemonic);

    setStorage(store);
    setAccounts(userAccounts);
    setClient(clientFromMnemonic);
  };

  const selectAccount = async (accountName: string) => {
    const mnemonic = await storage!.read_mnemonic(accountName);
    const clientFromMnemonic = await connectToValidator(mnemonic);
    setSelected(accountName);
    setClient(clientFromMnemonic);
  };

  const getFiatBalance = async (bal: number) => {
    const tokenPrice = await getTokenPrice('nym', currency);
    const fiatBal = tokenPrice.nym.gbp * bal;
    return fiatBal;
  };

  const getBalance = async () => {
    const bal = await client?.getBalance(client.address);
    if (bal) {
      const nym = unymToNym(Number(bal.amount));
      const fiat = await getFiatBalance(nym);
      setFiatBalance(fiat);
      setBalance(nym.toString());
    }
  };

  useEffect(() => {
    if (client) {
      getBalance();
    }
  }, [client]);

  const value = useMemo<TAppContext>(
    () => ({
      client,
      accounts,
      balance,
      fiatBalance,
      currency,
      denom,
      minorDenom,
      selectedAccount,
      storage,
      handleUnlockWallet,
      getBalance,
      setShowSeedForAccount,
      showSeedForAccount,
      setAccounts,
      selectAccount,
    }),
    [client, accounts, balance, fiatBalance, denom, minorDenom, selectedAccount, showSeedForAccount, storage],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
};

export const useAppContext = () => React.useContext(AppContext);
