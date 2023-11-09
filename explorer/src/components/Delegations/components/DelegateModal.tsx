import React, { useCallback, useContext, useState, useEffect, ChangeEvent } from 'react';
import { Box, Typography, SxProps, TextField } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, FeeDetails, DecCoin, decimalToFloatApproximation } from '@nymproject/types';
import { Console } from '../utils/console';
import { useGetFee } from '../hooks/useGetFee';
import { debounce } from 'lodash';
import { SimpleModal } from './SimpleModal';
import { ModalListItem } from './ModalListItem';

import { TPoolOption, checkTokenBalance, validateAmount, validateKey } from '../utils';

import { useChain } from '@cosmos-kit/react';
import { uNYMtoNYM } from '../utils';
import { ErrorModal } from './ErrorModal';
import { ConfirmTx } from './ConfirmTX';
import { BalanceWarning } from './FeeWarning';
import { getMixnodeStakeSaturation, simulateDelegateToMixnode, tryConvertIdentityToMixId } from '../requests';

const MIN_AMOUNT_TO_DELEGATE = 10;

export const DelegateModal: FCWithChildren<{
  open: boolean;
  onClose: () => void;
  onOk?: (
    mixId: number,
    identityKey: string,
    amount: DecCoin,
    tokenPool: TPoolOption,
    fee?: FeeDetails,
  ) => Promise<void>;
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
  rewardInterval,
  // accountBalance,
  estimatedReward,
  denom,
  profitMarginPercentage,
  nodeUptimePercentage,
  initialAmount,
  hasVestingContract,
  sx,
  backdropProps,
}) => {
  const [mixId, setMixId] = useState<number | undefined>();
  const [identityKey, setIdentityKey] = useState<string | undefined>(initialIdentityKey);
  const [amount, setAmount] = useState<string | undefined>(initialAmount);
  const [isValidated, setValidated] = useState<boolean>(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();
  // const [tokenPool, setTokenPool] = useState<TPoolOption>('balance');
  const [errorIdentityKey, setErrorIdentityKey] = useState<string>();
  const [mixIdError, setMixIdError] = useState<string>();

  const { fee, getFee, resetFeeState, feeError } = useGetFee();

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
  } = useChain('nyx');

  const [balance, setBalance] = useState<{
    status: 'loading' | 'success';
    data?: string;
  }>({ status: 'loading', data: undefined });

  useEffect(() => {
    const getBalance = async (walletAddress: string) => {
      const account = await getCosmWasmClient();
      const uNYMBalance = await account.getBalance(walletAddress, 'unym');
      const NYMBalance = uNYMtoNYM(uNYMBalance.amount).asString();

      setBalance({ status: 'success', data: NYMBalance });
    };

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

    if (amount && !(await validateAmount(amount, '0'))) {
      newValidatedValue = false;
      errorAmountMessage = 'Please enter a valid amount';
    }

    if (amount && Number(amount) < MIN_AMOUNT_TO_DELEGATE) {
      errorAmountMessage = `Min. delegation amount: ${MIN_AMOUNT_TO_DELEGATE} ${denom.toUpperCase()}`;
      newValidatedValue = false;
    }

    if (!amount?.length) {
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

  const handleOk = async () => {
    if (onOk && amount && identityKey && mixId) {
      onOk(mixId, identityKey, { amount, denom }, 'balance', fee);
    }
  };

  // const handleConfirm = async ({ mixId: id, value }: { mixId: number; value: DecCoin }) => {
  //   const SCWClient = await getSigningCosmWasmClient();

  //   console.log('SCWClient :>> ', SCWClient);
  // };

  const handleConfirm = async () => {
    const SCWClient = await getSigningCosmWasmClient();

    console.log('SCWClient :>> ', SCWClient);
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
    setAmount(newAmount.amount);

    if (onAmountChanged) {
      onAmountChanged(newAmount.amount);
    }
  };

  React.useEffect(() => {
    validate();
  }, [amount, identityKey, mixId]);

  // if (fee) {
  //   return (
  //     <ConfirmTx
  //       open
  //       header="Delegation details"
  //       fee={fee}
  //       onClose={onClose}
  //       onPrev={resetFeeState}
  //       onConfirm={handleOk}
  //     >
  //       {balance.data && fee?.amount?.amount && (
  //         <Box sx={{ my: 2 }}>
  //           <BalanceWarning fee={fee?.amount?.amount} tx={amount} />
  //         </Box>
  //       )}
  //       <ModalListItem label="Node identity key" value={identityKey} divider />
  //       <ModalListItem label="Amount" value={`${amount} ${denom.toUpperCase()}`} divider />
  //     </ConfirmTx>
  //   );
  // }

  // if (feeError) {
  //   return (
  //     <ErrorModal
  //       title="Something went wrong while calculating fee. Are you sure you entered a valid node address?"
  //       message={feeError}
  //       sx={sx}
  //       open={open}
  //       onClose={onClose}
  //     />
  //   );
  // }

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
      okDisabled={isValidated}
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
          initialValue={amount}
          autoFocus={Boolean(initialIdentityKey)}
          onChanged={handleAmountChanged}
          denom={denom}
          validationError={errorAmount}
        />
      </Box>
      <Box sx={{ mt: 3 }}>
        <ModalListItem label="Account balance" value={`${balance.data} NYM`} divider fontWeight={600} />
      </Box>

      {/* <ModalListItem label="Rewards payout interval" value={rewardInterval} hidden divider /> */}
      {/* <ModalListItem
        label="Node profit margin"
        value={`${profitMarginPercentage ? `${profitMarginPercentage}%` : '-'}`}
        hidden={profitMarginPercentage === undefined}
        divider
      />
      <ModalListItem
        label="Node avg. uptime"
        value={`${nodeUptimePercentage ? `${nodeUptimePercentage}%` : '-'}`}
        hidden={nodeUptimePercentage === undefined}
        divider
      /> */}

      {/* <ModalListItem
        label="Node est. reward per epoch"
        value={`${estimatedReward} ${denom.toUpperCase()}`}
        hidden
        divider
      /> */}
      <ModalListItem label="Est. fee for this transaction will be calculated in the next page" />
    </SimpleModal>
  );
};
