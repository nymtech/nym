import React, { useState } from 'react';
import { Box, Typography } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, MajorCurrencyAmount } from '@nymproject/types';
import { useGetFee } from 'src/hooks/useGetFee';
import { simulateDelegateToMixnode } from 'src/requests';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens, validateAmount, validateKey } from '../../utils';
import { TokenPoolSelector, TPoolOption } from '../TokenPoolSelector';
import { ConfirmTx } from '../ConfirmTX';

const MIN_AMOUNT_TO_DELEGATE = 10;

const checkTokenBalance = async (tokenPool: TPoolOption, amount: string) => {
  let hasEnoughFunds = false;
  if (tokenPool === 'locked') {
    hasEnoughFunds = await checkHasEnoughLockedTokens(amount);
  }

  if (tokenPool === 'balance') {
    hasEnoughFunds = await checkHasEnoughFunds(amount);
  }

  return hasEnoughFunds;
};

export const DelegateModal: React.FC<{
  open: boolean;
  onClose?: () => void;
  onOk?: (identityKey: string, amount: MajorCurrencyAmount, tokenPool: TPoolOption) => Promise<void>;
  identityKey?: string;
  onIdentityKeyChanged?: (identityKey: string) => void;
  onAmountChanged?: (amount: string) => void;
  header?: string;
  buttonText?: string;
  rewardInterval: string;
  accountBalance?: string;
  estimatedReward?: number;
  profitMarginPercentage?: number | null;
  nodeUptimePercentage?: number | null;
  currency: CurrencyDenom;
  initialAmount?: string;
  hasVestingContract: boolean;
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
  accountBalance,
  estimatedReward,
  currency,
  profitMarginPercentage,
  nodeUptimePercentage,
  initialAmount,
  hasVestingContract,
}) => {
  const [identityKey, setIdentityKey] = useState<string | undefined>(initialIdentityKey);
  const [amount, setAmount] = useState<string | undefined>(initialAmount);
  const [isValidated, setValidated] = useState<boolean>(false);
  const [errorAmount, setErrorAmount] = useState<string | undefined>();
  const [tokenPool, setTokenPool] = useState<TPoolOption>('balance');

  const { fee, getFee, resetFeeState } = useGetFee();

  const validate = async () => {
    let newValidatedValue = true;
    let errorMessage;

    if (!identityKey || !validateKey(identityKey, 32)) {
      newValidatedValue = false;
    }

    if (amount && !(await validateAmount(amount, '0'))) {
      newValidatedValue = false;
      errorMessage = 'Please enter a valid amount';
    }

    if (amount && Number(amount) < MIN_AMOUNT_TO_DELEGATE) {
      errorMessage = `Min. delegation amount: ${MIN_AMOUNT_TO_DELEGATE} ${currency}`;
      newValidatedValue = false;
    }

    if (!amount?.length) {
      newValidatedValue = false;
    }

    setErrorAmount(errorMessage);
    setValidated(newValidatedValue);
  };

  const handleOk = async () => {
    if (onOk && amount && identityKey) {
      onOk(identityKey, { amount, denom: currency }, tokenPool);
    }
  };

  const handleConfirm = async ({ identity, value }: { identity: string; value: MajorCurrencyAmount }) => {
    const hasEnoughTokens = await checkTokenBalance(tokenPool, value.amount);

    if (!hasEnoughTokens) {
      setErrorAmount('Not enough funds');
      return;
    }

    if (tokenPool === 'locked') {
      getFee(simulateDelegateToMixnode, { identity, amount: value });
    }

    if (tokenPool === 'balance') {
      getFee(simulateDelegateToMixnode, { identity, amount: value });
    }
  };

  const handleIdentityKeyChanged = (newIdentityKey: string) => {
    setIdentityKey(newIdentityKey);
    if (onIdentityKeyChanged) {
      onIdentityKeyChanged(newIdentityKey);
    }
  };

  const handleAmountChanged = (newAmount: MajorCurrencyAmount) => {
    setAmount(newAmount.amount);
    if (onAmountChanged) {
      onAmountChanged(newAmount.amount);
    }
  };

  React.useEffect(() => {
    validate();
  }, [amount, identityKey]);

  if (fee) {
    return (
      <ConfirmTx
        open
        header="Delegation details"
        fee={fee}
        onClose={onClose}
        onPrev={resetFeeState}
        onConfirm={handleOk}
      >
        <ModalListItem label="Node identity key" value={identityKey} divider />
        <ModalListItem label="Amount" value={`${amount} ${currency}`} divider />
      </ConfirmTx>
    );
  }

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={async () => {
        if (identityKey && amount) {
          handleConfirm({ identity: identityKey, value: { amount, denom: currency } });
        }
      }}
      header={header || 'Delegate'}
      subHeader="Delegate to mixnode"
      okLabel={buttonText || 'Delegate stake'}
      okDisabled={!isValidated}
    >
      <IdentityKeyFormField
        required
        fullWidth
        placeholder="Node identity key"
        onChanged={handleIdentityKeyChanged}
        initialValue={identityKey}
        readOnly={Boolean(initialIdentityKey)}
        textFieldProps={{
          autoFocus: !initialIdentityKey,
        }}
      />
      <Box display="flex" gap={2} alignItems="center" sx={{ mt: 2 }}>
        {hasVestingContract && <TokenPoolSelector disabled={false} onSelect={(pool) => setTokenPool(pool)} />}
        <CurrencyFormField
          required
          fullWidth
          placeholder="Amount"
          initialValue={amount}
          autoFocus={Boolean(initialIdentityKey)}
          onChanged={handleAmountChanged}
        />
      </Box>
      <Typography component="div" textAlign="right" variant="caption" sx={{ color: 'error.main' }}>
        {errorAmount}
      </Typography>
      <Box sx={{ mt: 3 }}>
        <ModalListItem label="Account balance" value={accountBalance} divider />
      </Box>

      <ModalListItem label="Rewards payout interval" value={rewardInterval} hidden divider />
      <ModalListItem
        label="Node profit margin"
        value={`${profitMarginPercentage ? `${profitMarginPercentage}%` : '-'}`}
        hidden={profitMarginPercentage === undefined}
        divider
      />
      <ModalListItem
        label="Node uptime"
        value={`${nodeUptimePercentage ? `${nodeUptimePercentage}%` : '-'}`}
        hidden={nodeUptimePercentage === undefined}
        divider
      />

      <ModalListItem label="Node est. reward per epoch" value={`${estimatedReward} ${currency}`} hidden divider />
    </SimpleModal>
  );
};
