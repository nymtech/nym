import React, { useState } from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, MajorCurrencyAmount } from '@nymproject/types';
import { getGasFee } from 'src/requests';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from './ModalListItem';
import { validateKey } from '../../utils';
import { TokenPoolSelector, TPoolOption } from '../TokenPoolSelector';
import { getMixnodeStakeSaturation } from '../../requests';

const MIN_AMOUNT_TO_DELEGATE = 10;

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
  feeOverride,
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
  const [fee, setFee] = useState<string>();
  const [nodeSaturation, setNodeSaturation] = useState<number>(0);

  const getFee = async () => {
    if (feeOverride) setFee(feeOverride);
    else {
      const res = await getGasFee('BondMixnode');
      setFee(res.amount);
    }
  };

  const validate = () => {
    let newValidatedValue = true;
    if (!identityKey || !validateKey(identityKey, 32)) {
      newValidatedValue = false;
    }
    if (amount && Number(amount) < MIN_AMOUNT_TO_DELEGATE) {
      setErrorAmount(`Min. delegation amount: ${MIN_AMOUNT_TO_DELEGATE} ${currency}`);
      newValidatedValue = false;
    } else {
      setErrorAmount(undefined);
    }
    setValidated(newValidatedValue);
  };

  const handleOk = () => {
    if (onOk && amount && identityKey) {
      onOk(identityKey, { amount, denom: currency }, tokenPool);
    }
  };

  const setSaturation = async (identityKey: string) => {
    setValidated(false);
    const newSaturation = await getMixnodeStakeSaturation(identityKey);
    if (newSaturation) {
      setValidated(true);
      const saturationPercentage = Math.round(newSaturation.saturation * 100);
      setNodeSaturation(saturationPercentage);
    }
  };

  const handleIdentityKeyChanged = async (newIdentityKey: string) => {
    if (newIdentityKey) {
      setSaturation(newIdentityKey);
    }
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

  const handleIdentityValidation = (isValid: boolean) => {
    if (!isValid) {
      setNodeSaturation(0);
      setValidated(false);
      return;
    }
    setValidated(true);
  };

  React.useEffect(() => {
    validate();
  }, [amount, identityKey]);

  React.useEffect(() => {
    getFee();
  }, []);

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={handleOk}
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
        onValidate={handleIdentityValidation}
        initialValue={initialIdentityKey}
        readOnly={Boolean(initialIdentityKey)}
        textFieldProps={{
          autoFocus: !initialIdentityKey,
        }}
        saturation={nodeSaturation}
      />
      <Box display="flex" gap={2} alignItems="center" sx={{ mt: 2 }}>
        {hasVestingContract && <TokenPoolSelector disabled={false} onSelect={(pool) => setTokenPool(pool)} />}
        <CurrencyFormField
          required
          fullWidth
          placeholder="Amount"
          initialValue={initialAmount}
          autoFocus={Boolean(initialIdentityKey)}
          onChanged={handleAmountChanged}
        />
      </Box>
      <Typography component="div" textAlign="right" variant="caption" sx={{ color: 'error.main' }}>
        {errorAmount}
      </Typography>
      <Stack direction="row" justifyContent="space-between" my={3}>
        <Typography fontWeight={600}>Account balance</Typography>
        <Typography fontWeight={600}>{accountBalance}</Typography>
      </Stack>
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
      <Stack direction="row" justifyContent="space-between" mt={4}>
        <Typography fontSize="smaller" color={(theme) => theme.palette.nym.fee}>
          Est. fee for this transaction:
        </Typography>
        <Typography fontSize="smaller" color={(theme) => theme.palette.nym.fee}>
          {fee} {currency}
        </Typography>
      </Stack>
    </SimpleModal>
  );
};
