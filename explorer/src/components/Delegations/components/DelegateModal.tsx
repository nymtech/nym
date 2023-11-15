import React, { useState, useEffect, ChangeEvent } from 'react';
import { Box, Typography, SxProps, TextField } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, DecCoin } from '@nymproject/types';
import { SimpleModal } from './SimpleModal';
import { ModalListItem } from './ModalListItem';
import { Console, urls, validateAmount } from '../utils';
import { useChain } from '@cosmos-kit/react';
import { StdFee } from '@cosmjs/amino';
import { ExecuteResult } from '@cosmjs/cosmwasm-stargate';
import { uNYMtoNYM } from '../utils';
import { DelegationModalProps } from './DelegationModal';

const MIN_AMOUNT_TO_DELEGATE = 10;
const MIXNET_CONTRACT_ADDRESS = 'n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr';
// const sandboxContractAddress = 'n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav';

export const DelegateModal: FCWithChildren<{
  open: boolean;
  onClose: () => void;
  onOk?: (delegationModalProps: DelegationModalProps) => void;
  identityKey?: string;
  onIdentityKeyChanged?: (identityKey: string) => void;
  onAmountChanged?: (amount: string) => void;
  header?: string;
  buttonText?: string;
  rewardInterval?: string;
  // accountBalance?: string;
  estimatedReward?: number;
  profitMarginPercentage?: string | null;
  nodeUptimePercentage?: number | null;
  denom: CurrencyDenom;
  initialAmount?: string;
  hasVestingContract?: boolean;
  sx?: SxProps;
  backdropProps?: object;
}> = ({
  open,
  onIdentityKeyChanged,
  onAmountChanged,
  onClose,
  onOk,
  header,
  buttonText,
  identityKey: initialIdentityKey,
  // accountBalance,
  denom,
  sx,
  backdropProps,
}) => {
  const [mixId, setMixId] = useState<number | undefined>();
  const [identityKey, setIdentityKey] = useState<string | undefined>(initialIdentityKey);
  const [amount, setAmount] = useState<DecCoin | undefined>();
  const [isValidated, setValidated] = useState<boolean>(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();
  const [errorIdentityKey, setErrorIdentityKey] = useState<string>();
  const [mixIdError, setMixIdError] = useState<string>();
  const [cosmWasmSignerClient, setCosmWasmSignerClient] = useState<any>();
  const [balance, setBalance] = useState<{
    status: 'loading' | 'success';
    data?: string;
  }>({ status: 'loading', data: undefined });

  const {
    username,
    connect,
    disconnect,
    wallet,
    openView,
    address,
    getCosmWasmClient,
    isWalletConnected,
    getSigningCosmWasmClient,
    estimateFee,
  } = useChain('nyx');

  useEffect(() => {
    const getClient = async () => {
      await getSigningCosmWasmClient()
        .then((res) => {
          setCosmWasmSignerClient(res);
          console.log('res :>> ', res);
        })
        .catch((e) => console.log('e :>> ', e));
    };

    isWalletConnected && getClient();
  }, [isWalletConnected]);

  const getBalance = async (walletAddress: string) => {
    const account = await getCosmWasmClient();
    const uNYMBalance = await account.getBalance(walletAddress, 'unym');
    const NYMBalance = uNYMtoNYM(uNYMBalance.amount).asString();

    setBalance({ status: 'success', data: NYMBalance });
  };
  useEffect(() => {
    if (address) {
      getBalance(address);
    }
  }, [address, getCosmWasmClient]);

  const validate = async () => {
    let newValidatedValue = true;
    let errorAmountMessage;
    let errorIdentityKeyMessage;

    if (!identityKey) {
      newValidatedValue = false;
      errorIdentityKeyMessage = 'Please enter a valid identity key';
    }

    if (amount && !(await validateAmount(amount.amount, '0'))) {
      newValidatedValue = false;
      errorAmountMessage = 'Please enter a valid amount';
    }

    if (amount && Number(amount) < MIN_AMOUNT_TO_DELEGATE) {
      errorAmountMessage = `Min. delegation amount: ${MIN_AMOUNT_TO_DELEGATE} ${denom.toUpperCase()}`;
      newValidatedValue = false;
    }

    if (!amount?.amount.length) {
      newValidatedValue = false;
    }

    if (!mixId) {
      newValidatedValue = false;
    }

    if (amount && balance.data && +balance.data - +amount <= 0) {
      errorAmountMessage = 'Not enough funds';
      newValidatedValue = false;
    }

    setErrorIdentityKey(errorIdentityKeyMessage);
    if (mixIdError && !errorIdentityKeyMessage) {
      setErrorIdentityKey(mixIdError);
    }
    setErrorAmount(errorAmountMessage);
    setValidated(newValidatedValue);
  };

  const delegateToMixnode = async (
    {
      mixId,
    }: {
      mixId: number;
    },
    fee: number | StdFee | 'auto' = 'auto',
    memo?: string,
    funds?: DecCoin[],
  ): Promise<ExecuteResult> => {
    console.log('cosmWasmSignerClient :>> ', cosmWasmSignerClient);
    const amount = (Number(funds![0].amount) * 1000000).toString();
    const uNymFunds = [{ amount: amount, denom: 'unym' }];
    return await cosmWasmSignerClient.execute(
      address,
      MIXNET_CONTRACT_ADDRESS,
      {
        delegate_to_mixnode: {
          mix_id: mixId,
        },
      },
      fee,
      memo,
      uNymFunds,
    );
  };

  const handleConfirm = async () => {
    const memo: string = 'test delegation';
    const fee = { gas: '1000000', amount: [{ amount: '25000', denom: 'unym' }] };

    if (mixId && amount && onOk && cosmWasmSignerClient) {
      onOk({
        status: 'loading',
        action: 'delegate',
      });
      try {
        await delegateToMixnode({ mixId }, fee, memo, [amount]).then((res) => {
          console.log('res :>> ', res);
        });
        const tx = await delegateToMixnode({ mixId }, fee, memo, [amount]);

        onOk({
          status: 'success',
          action: 'delegate',
          message: 'This operation can take up to one hour to process',
          transactions: [
            { url: `${urls('MAINNET').blockExplorer}/transaction/${tx.transactionHash}`, hash: tx.transactionHash },
          ],
        });
      } catch (e) {
        Console.error('Failed to addDelegation', e);
        onOk({
          status: 'error',
          action: 'delegate',
          message: (e as Error).message,
        });
      }
    }
  };

  const handleIdentityKeyChanged = (newIdentityKey: string) => {
    setIdentityKey(newIdentityKey);

    if (onIdentityKeyChanged) {
      onIdentityKeyChanged(newIdentityKey);
    }
  };

  const handleMixIDChanged = (event: ChangeEvent<HTMLInputElement>) => {
    const newValue = event.target.value;
    setMixId(Number(newValue));
  };

  const handleAmountChanged = (newAmount: DecCoin) => {
    setAmount(newAmount);

    if (onAmountChanged) {
      onAmountChanged(newAmount.amount);
    }
  };

  React.useEffect(() => {
    validate();
  }, [amount, identityKey, mixId]);

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      // onOk={async () => {
      //   if (mixId && amount) {
      //     handleConfirm({ mixId, value: { amount, denom } });
      //   }
      // }}
      onOk={async () => handleConfirm()}
      header={header || 'Delegate'}
      okLabel={buttonText || 'Delegate stake'}
      okDisabled={!isValidated}
      sx={sx}
      backdropProps={backdropProps}
    >
      <Box sx={{ mt: 3 }} gap={2}>
        <IdentityKeyFormField
          required
          fullWidth
          label="Node identity key"
          onChanged={handleIdentityKeyChanged}
          initialValue={identityKey}
          readOnly={Boolean(initialIdentityKey)}
          textFieldProps={{
            autoFocus: !initialIdentityKey,
          }}
          showTickOnValid={false}
        />
        <Typography
          component="div"
          textAlign="left"
          variant="caption"
          sx={{ color: 'error.main', mx: 2, mt: errorIdentityKey && 1 }}
        >
          {errorIdentityKey}
        </Typography>
      </Box>
      <Box sx={{ mt: 3 }} gap={2}>
        <TextField
          fullWidth={true}
          required
          label={'MixID'}
          error={mixIdError !== undefined}
          helperText={mixIdError}
          onChange={handleMixIDChanged}
          InputLabelProps={{ shrink: true }}
        />
      </Box>
      <Box display="flex" gap={2} alignItems="center" sx={{ mt: 3 }}>
        <CurrencyFormField
          required
          fullWidth
          label="Amount"
          // initialValue={amount}
          autoFocus={Boolean(initialIdentityKey)}
          onChanged={handleAmountChanged}
          denom={denom}
          validationError={errorAmount}
        />
      </Box>
      <Box sx={{ mt: 3 }}>
        <ModalListItem label="Account balance" value={`${balance.data} NYM`} divider fontWeight={600} />
      </Box>

      <ModalListItem label="Est. fee for this transaction will be calculated in the next page" />
    </SimpleModal>
  );
};
