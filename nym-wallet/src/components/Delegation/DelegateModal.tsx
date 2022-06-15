import React, { useEffect, useState } from 'react';
import { Box, Stack, Typography } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, MajorCurrencyAmount } from '@nymproject/types';
import { getGasFee } from 'src/requests';
import { Console } from 'src/utils/console';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from './ModalListItem';
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
  const [fee, setFee] = useState<string>();
  const [tokenPool, setTokenPool] = useState<TPoolOption>('balance');
  const [isValidated, setValidated] = useState<boolean>(false);
  const [errorAmount, setErrorAmount] = useState<string>();
  const [errorNodeSaturation, setErrorNodeSaturation] = useState<string>();
  const [errorIdentityKey, setErrorIdentityKey] = useState<string>();

  const getFee = async () => {
    if (feeOverride) setFee(feeOverride);
    else {
      const res = await getGasFee('BondMixnode');
      setFee(res.amount);
    }
  };

  const handleCheckStakeSaturation = async (identity: string) => {
    setErrorNodeSaturation(undefined);

    try {
      const newSaturation = await getMixnodeStakeSaturation(identity);
      if (newSaturation && newSaturation.saturation > 1) {
        const saturationPercentage = Math.round(newSaturation.saturation * 100);
        setErrorNodeSaturation(`This node is over saturated (${saturationPercentage}%), please select another node`);
      }
    } catch (e) {
      Console.error('Error fetching the saturation, error:', e);
      setErrorNodeSaturation(undefined);
    }
  };

  const validateIdentityKey = async (isValid: boolean) => {
    if (!isValid) {
      setErrorIdentityKey('Identity key is invalid');
      setErrorNodeSaturation(undefined);
    } else {
      setErrorIdentityKey(undefined);
      await handleCheckStakeSaturation(identityKey!);
    }
  };

  const validateAmount = (newValue: MajorCurrencyAmount) => {
    setErrorAmount(undefined);

    if (newValue.amount && Number(newValue.amount) < MIN_AMOUNT_TO_DELEGATE) {
      setErrorAmount(`Min. delegation amount: ${MIN_AMOUNT_TO_DELEGATE} ${currency}`);
    }

    if (!newValue.amount) {
      setErrorAmount('Amount required');
    }
  };

  const handleOk = () => {
    if (onOk && amount && identityKey) {
      onOk(identityKey, { amount, denom: currency }, tokenPool);
    }
  };

  const handleIdentityKeyChanged = async (newIdentityKey: string) => {
    setIdentityKey(newIdentityKey);

    if (onIdentityKeyChanged) {
      onIdentityKeyChanged(newIdentityKey);
    }
  };

  const handleAmountChanged = (newAmount: MajorCurrencyAmount) => {
    setAmount(newAmount.amount);
    validateAmount(newAmount);

    if (onAmountChanged) {
      onAmountChanged(newAmount.amount);
    }
  };

  useEffect(() => {
    getFee();
  }, []);

  useEffect(() => {
    if (!!errorIdentityKey || !!errorAmount || errorNodeSaturation) setValidated(false);
    else setValidated(true);
  }, [errorIdentityKey, errorAmount, errorNodeSaturation]);

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
        onValidate={validateIdentityKey}
        initialValue={initialIdentityKey}
        readOnly={Boolean(initialIdentityKey)}
        textFieldProps={{
          autoFocus: !initialIdentityKey,
        }}
      />
      <Typography
        component="div"
        textAlign="left"
        variant="caption"
        sx={{ color: 'error.main', mx: '14px', mt: '3px' }}
      >
        {errorNodeSaturation}
      </Typography>
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
      <Typography
        component="div"
        textAlign="left"
        variant="caption"
        sx={{ color: 'error.main', mx: '14px', mt: '3px' }}
      >
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
