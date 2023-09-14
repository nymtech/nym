import React, { useState, useEffect, useCallback } from 'react';
import { contracts } from '@nymproject/contract-clients';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { Coin, GasPrice } from '@cosmjs/stargate';
import Button from '@mui/material/Button';
import Input from '@mui/material/Input';
import Paper from '@mui/material/Paper';
import Box from '@mui/material/Box';
import { settings } from './client';
import { TextField, Typography } from '@mui/material';
import Divider from '@mui/material/Divider';
import Table from '@mui/material/Table';
import { TableBody, TableCell, TableHead, TableRow } from '@mui/material';

const signerAccount = async (mnemonic) => {
  // create a wallet to sign transactions with the mnemonic
  const signer = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: 'n',
  });

  return signer;
};

const fetchSignerCosmosWasmClient = async (mnemonic) => {
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

export const Wallet = () => {
  const [mnemonic, setMnemonic] = useState<string>();
  const [signerCosmosWasmClient, setSignerCosmosWasmClient] = useState<any>();
  const [signerClient, setSignerClient] = useState<any>();
  const [account, setAccount] = useState<string>();
  const [delegations, setDelegations] = useState<any>();
  const [log, setLog] = useState<React.ReactNode[]>([]);
  const [balance, setBalance] = useState<Coin>();
  const [tokensToSend, setTokensToSend] = useState<string>();
  const [recipientAddress, setRecipientAddress] = useState<string>('');
  const [delegationNodeId, setDelegationNodeId] = useState<number>();
  const [amountToBeDelegated, setAmountToBeDelegated] = useState<number>();

  const getBalance = useCallback(async () => {
    try {
      const newBalance = await signerCosmosWasmClient?.getBalance(account, 'unym');
      setBalance(newBalance);
    } catch (error) {
      console.error(error);
    }
  }, [account, signerCosmosWasmClient]);

  const getSignerAccount = async () => {
    try {
      const signer = await signerAccount(mnemonic);
      const accounts = await signer.getAccounts();
      if (accounts[0]) {
        setAccount(accounts[0].address);
      }
    } catch (error) {
      console.error(error);
    }
  };

  const getClients = async () => {
    try {
      setSignerCosmosWasmClient(await fetchSignerCosmosWasmClient(mnemonic));
      setSignerClient(await fetchSignerClient(mnemonic));
    } catch (error) {
      console.error(error);
    }
  };

  const getDelegations = useCallback(async () => {
    const newDelegations = await signerClient.getDelegatorDelegations({
      delegator: settings.address,
    });
    setDelegations(newDelegations);
  }, [signerClient]);

  useEffect(() => {
    if (mnemonic) {
      getSignerAccount();
      getClients();
    }
  }, [mnemonic]);

  useEffect(() => {
    if (account && signerCosmosWasmClient) {
      if (!balance) {
        getBalance();
      }
    }
  }, [account, signerCosmosWasmClient, balance, getBalance]);

  useEffect(() => {
    if (signerClient && !delegations) {
      console.log('getDelegations');
      getDelegations();
    }
  }, [signerClient, getDelegations, delegations]);

  const doUndelegateAll = async () => {
    if (!signerClient) {
      return;
    }
    console.log('delegations', delegations);
    try {
      // eslint-disable-next-line no-restricted-syntax
      for (const delegation of delegations.delegations) {
        // eslint-disable-next-line no-await-in-loop
        await signerClient.undelegateFromMixnode({ mixId: delegation.mix_id }, 'auto');
      }
    } catch (error) {
      console.error(error);
    }
  };

  const doDelegate = async ({ mixId, amount }: { mixId: number; amount: number }) => {
    if (!signerClient) {
      return;
    }
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
  };
  // End delegate

  // Sending tokens
  const doSendTokens = async () => {
    const memo = 'test sending tokens';

    try {
      const res = await signerCosmosWasmClient.sendTokens(
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
  };
  // End send tokens

  // Withdraw Rewards
  const doWithdrawRewards = async () => {
    const delegatorAddress = '';
    const validatorAdress = '';
    const memo = 'test sending tokens';
    const res = await signerCosmosWasmClient.withdrawRewards(delegatorAddress, validatorAdress, 'auto', memo);
    console.log({ res });
  };

  console.log(delegations?.delegations);

  return (
    <Box padding={3}>
      <Paper style={{ marginTop: '1rem', padding: '1rem' }}>
        <Typography variant="h5" textAlign="center">
          Basic Wallet
        </Typography>
        <Box padding={3}>
          <Typography variant="h6">Your account</Typography>
          <Box marginY={3}>
            <Typography variant="body1" marginBottom={3}>
              Enter the mnemonic
            </Typography>
            <TextField type="text" placeholder="mnemonic" onChange={(e) => setMnemonic(e.target.value)} fullWidth />
          </Box>
          {account && balance ? (
            <Box>
              <Typography variant="body1">Address: {account}</Typography>
              <Typography variant="body1">
                Balance: {balance?.amount} {balance?.denom}
              </Typography>
            </Box>
          ) : (
            <Box>
              <Typography variant="body1">Please, enter your nemonic to receive your account info</Typography>
            </Box>
          )}
        </Box>
        <Divider />
        <Box padding={3}>
          <Typography variant="h6">Send Tokens</Typography>
          <Box marginTop={3} display="flex" flexDirection="column">
            <Input type="text" placeholder="Recipient Address" onChange={(e) => setRecipientAddress(e.target.value)} />
            <Box marginY={3} display="flex" justifyContent="space-between">
              <Input
                type="number"
                placeholder="Amount"
                onChange={(e) => {
                  setTokensToSend(e.target.value);
                }}
              />
              <Button variant="outlined" onClick={() => doSendTokens()}>
                SendTokens
              </Button>
            </Box>
          </Box>
        </Box>
        <Divider />
        <Box padding={3}>
          <Typography variant="h6">Delegations</Typography>
          <Box marginY={3}>
            <Box marginY={3} display="flex" flexDirection="column">
              <Typography marginBottom={3} variant="body1">
                Make a delegation
              </Typography>
              <Input
                type="number"
                placeholder="Mixnode ID"
                onChange={(e) => setDelegationNodeId(parseInt(e.target.value, 10))}
              />
              <Box marginTop={3} display="flex" justifyContent="space-between">
                <Input
                  type="number"
                  placeholder="Amount"
                  onChange={(e) => setAmountToBeDelegated(parseInt(e.target.value, 10))}
                />
                <Button
                  variant="outlined"
                  onClick={() => doDelegate({ mixId: delegationNodeId, amount: amountToBeDelegated })}
                >
                  Delegate
                </Button>
              </Box>
            </Box>
          </Box>
          <Box marginTop={3}>
            <Typography variant="body1">Your delegations</Typography>
            <Box marginBottom={3} display="flex" flexDirection="column">
              {!delegations?.delegations?.length ? (
                <Typography>You don't have delegations</Typography>
              ) : (
                <Box>
                  <Table size="small">
                    <TableHead>
                      <TableRow>
                        <TableCell>MixId</TableCell>
                        <TableCell>Owner</TableCell>
                        <TableCell>Amount</TableCell>
                        <TableCell>Cumulative Reward Ratio</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {delegations?.delegations.map((delegation: any) => (
                        <TableRow>
                          <TableCell>{delegation.mix_id}</TableCell>
                          <TableCell>{delegation.owner}</TableCell>
                          <TableCell>{delegation.amount.amount}</TableCell>
                          <TableCell>{delegation.cumulative_reward_ratio}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </Box>
              )}
            </Box>
            {delegations && (
              <Box marginBottom={3}>
                <Button variant="outlined" onClick={doUndelegateAll}>
                  Undelegate All
                </Button>
              </Box>
            )}
            <Box>
              <Button variant="outlined" onClick={() => doWithdrawRewards()}>
                Withdraw rewards
              </Button>
            </Box>
          </Box>
        </Box>
      </Paper>

      <Box marginTop={3}>
        <Typography variant="h5">Transaction Logs:</Typography>
        {log}
      </Box>
    </Box>
  );
};
