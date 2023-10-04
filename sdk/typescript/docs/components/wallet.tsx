import React, { useCallback, useEffect, useState } from 'react';
import { contracts } from '@nymproject/contract-clients';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { Coin, GasPrice } from '@cosmjs/stargate';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { settings } from './client';
import { ConnectWallet } from './wallet/connect';
import { SendTokes } from './wallet/sendTokens';
import { Delegations } from './wallet/delegations';

const signerAccount = async (mnemonic) => {
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

export const Wallet = ({ type }: { type: 'connect' | 'sendTokens' | 'delegations' }) => {
  const [mnemonic, setMnemonic] = useState<string>();
  const [signerCosmosWasmClient, setSignerCosmosWasmClient] = useState<any>();
  const [signerClient, setSignerClient] = useState<any>();
  const [account, setAccount] = useState<string>();
  const [accountLoading, setAccountLoading] = useState<boolean>(false);
  const [clientLoading, setClientLoading] = useState<boolean>(false);
  const [balance, setBalance] = useState<Coin>();
  const [balanceLoading, setBalanceLoading] = useState<boolean>(false);
  const [log, setLog] = useState<React.ReactNode[]>([]);
  const [sendingTokensLoader, setSendingTokensLoader] = useState<boolean>(false);
  const [delegations, setDelegations] = useState<any>();
  const [recipientAddress, setRecipientAddress] = useState<string>('');
  const [delegationLoader, setDelegationLoader] = useState<boolean>(false);
  const [undeledationLoader, setUndeledationLoader] = useState<boolean>(false);
  const [withdrawLoading, setWithdrawLoading] = useState<boolean>(false);
  const [connectButtonText, setConnectButtonText] = useState<string>('Connect');

  const getBalance = useCallback(async () => {
    setBalanceLoading(true);
    try {
      const newBalance = await signerCosmosWasmClient?.getBalance(account, 'unym');
      setBalance(newBalance);
    } catch (error) {
      console.error(error);
    }
    setBalanceLoading(false);
  }, [account, signerCosmosWasmClient]);

  const getSignerAccount = async () => {
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

  const getClients = async () => {
    setClientLoading(true);
    try {
      setSignerCosmosWasmClient(await fetchSignerCosmosWasmClient(mnemonic));
      setSignerClient(await fetchSignerClient(mnemonic));
    } catch (error) {
      console.error(error);
    }
    setClientLoading(false);
  };

  const getDelegations = useCallback(async () => {
    const newDelegations = await signerClient.getDelegatorDelegations({
      delegator: settings.address,
    });
    setDelegations(newDelegations);
  }, [signerClient]);

  const connect = () => {
    getSignerAccount();
    getClients();
  };

  // Start Undelgate All
  const doUndelegateAll = async () => {
    if (!signerClient) {
      return;
    }
    setUndeledationLoader(true);
    try {
      // eslint-disable-next-line no-restricted-syntax
      for (const delegation of delegations.delegations) {
        // eslint-disable-next-line no-await-in-loop
        await signerClient.undelegateFromMixnode({ mixId: delegation.mix_id }, 'auto');
      }
    } catch (error) {
      console.error(error);
    }
    setUndeledationLoader(false);
  };
  // End Undelgate All

  // Start Delegate
  const doDelegate = async ({ mixId, amount }: { mixId: number; amount: number }) => {
    if (!signerClient) {
      return;
    }
    setDelegationLoader(true);
    try {
      const res = await signerClient.delegateToMixnode({ mixId }, 'auto', undefined, [
        { amount: `${amount}`, denom: 'unym' },
      ]);
      console.log('res', res);
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
  // End delegate

  // Sending tokens
  const doSendTokens = async (amount: string) => {
    const memo = 'test sending tokens';
    setSendingTokensLoader(true);
    try {
      const res = await signerCosmosWasmClient.sendTokens(
        account,
        recipientAddress,
        [{ amount, denom: 'unym' }],
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
    setSendingTokensLoader(false);
  };
  // End send tokens

  // Start Withdraw Rewards
  const doWithdrawRewards = async () => {
    const delegatorAddress = '';
    const validatorAdress = '';
    const memo = 'test sending tokens';
    setWithdrawLoading(true);
    try {
      const res = await signerCosmosWasmClient.withdrawRewards(delegatorAddress, validatorAdress, 'auto', memo);
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
  // End Withdraw Rewards

  useEffect(() => {
    if (account && signerCosmosWasmClient) {
      if (!balance) {
        setBalanceLoading(true);
        getBalance();
        setBalanceLoading(false);
      }
    }
  }, [account, signerCosmosWasmClient, balance, getBalance]);

  useEffect(() => {
    if (signerClient && !delegations) {
      console.log('getDelegations');
      getDelegations();
    }
  }, [signerClient, getDelegations, delegations]);

  useEffect(() => {
    if (accountLoading || clientLoading || balanceLoading) {
      setConnectButtonText('Loading...');
    } else if (balance) {
      setConnectButtonText('Connected');
    }
    setConnectButtonText('Connect');
  }, [accountLoading, clientLoading, balanceLoading]);

  return (
    <Box padding={3}>
      {type === 'connect' && (
        <ConnectWallet
          setMnemonic={setMnemonic}
          connect={connect}
          mnemonic={mnemonic}
          accountLoading={accountLoading}
          clientLoading={clientLoading}
          balanceLoading={balanceLoading}
          account={account}
          balance={balance}
          connectButtonText={connectButtonText}
        />
      )}
      {type === 'sendTokens' && (
        <SendTokes
          setRecipientAddress={setRecipientAddress}
          doSendTokens={doSendTokens}
          sendingTokensLoader={sendingTokensLoader}
        />
      )}
      {type === 'delegations' && (
        <Delegations
          delegations={delegations}
          doDelegate={doDelegate}
          delegationLoader={delegationLoader}
          undeledationLoader={undeledationLoader}
          doUndelegateAll={doUndelegateAll}
          doWithdrawRewards={doWithdrawRewards}
          withdrawLoading={withdrawLoading}
        />
      )}
      {log.length > 0 && (
        <Box marginTop={3}>
          <Typography variant="h5">Transaction Logs:</Typography>
          {log}
        </Box>
      )}
    </Box>
  );
};
