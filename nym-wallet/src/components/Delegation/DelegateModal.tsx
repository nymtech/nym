import React, { useState } from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, FeeDetails, MajorCurrencyAmount } from '@nymproject/types';
import { getGasFee, simulateDelegateToMixnode } from 'src/requests';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens, validateAmount, validateKey } from '../../utils';
import { TokenPoolSelector, TPoolOption } from '../TokenPoolSelector';
import { ConfirmTx } from '../ConfirmTX';
import { Console } from 'src/utils/console';

const MIN_AMOUNT_TO_DELEGATE = 10;

type confirmationResponseType = {
  error?: string;
  fees?: FeeDetails;
};

const handleConfirmWithBalance = async ({
  identityKey,
  amount,
}: {
  identityKey: string;
  amount: MajorCurrencyAmount;
}) => {
  const response: confirmationResponseType = {};
  const hasEnoughTokens = await checkHasEnoughFunds(amount.amount);

  try {
    if (!hasEnoughTokens) {
      response.error = 'Not enough funds';
    } else {
      const fees = await simulateDelegateToMixnode({ identity: identityKey, amount });
      response.fees = fees;
    }
    return response;
  } catch (e) {
    Console.error(e);
    response.fees = undefined;
    response.error = 'An error occurred. Please check the address and amount are correct';
    return response;
  }
};

const handleConfirmWithLocked = async ({
  identityKey,
  amount,
}: {
  identityKey: string;
  amount: MajorCurrencyAmount;
}) => {
  const response: confirmationResponseType = {};
  const hasEnoughTokens = await checkHasEnoughLockedTokens(amount.amount);

  try {
    if (!hasEnoughTokens) {
      response.error = 'Not enough funds';
    } else {
      const fees = await simulateDelegateToMixnode({ identity: identityKey, amount });
      response.fees = fees;
    }
    return response;
  } catch (e) {
    Console.error(e);
    response.fees = undefined;
    response.error = 'An error occurred. Please check the address and amount are correct';
    return response;
  }
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
  feeOverride?: string;
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
  const [fee, setFee] = useState<FeeDetails>();

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

  const handleConfirm = async ({ identityKey, amount }: { identityKey: string; amount: MajorCurrencyAmount }) => {
    let response: confirmationResponseType = {};

    if (tokenPool === 'locked') {
      response = await handleConfirmWithLocked({ identityKey, amount });
    } else {
      response = await handleConfirmWithBalance({ identityKey, amount });
    }

    if (response.error) {
      setErrorAmount(response.error);
    }

    if (!response.error && response.fees) {
      setFee(response.fees);
      setErrorAmount(undefined);
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

  if (fee?.amount) {
    return (
      <ConfirmTx
        open
        header="Delegation details"
        fee={fee.amount}
        onClose={onClose}
        onPrev={() => setFee(undefined)}
        onConfirm={handleOk}
        currency={currency}
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
          handleConfirm({ identityKey, amount: { amount, denom: currency } });
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
