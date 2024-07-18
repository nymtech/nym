import React, { useContext, useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { Box, FormHelperText, Stack, TextField, Typography } from '@mui/material';
import { yupResolver } from '@hookform/resolvers/yup/dist/yup';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom } from '@nymproject/types';
import { amountSchema } from './mixnodeValidationSchema';
import { MixnodeAmount } from '../../../pages/bonding/types';
import { AppContext } from '../../../context';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens } from '../../../utils';
import { TokenPoolSelector } from '../../TokenPoolSelector';
import { ModalListItem } from '../../Modals/ModalListItem';

const MixnodeAmountForm = ({
  amountData,
  hasVestingTokens,
  denom,
  onNext,
}: {
  amountData: MixnodeAmount;
  hasVestingTokens: boolean;
  denom: CurrencyDenom;
  onNext: (data: MixnodeAmount) => void;
}) => {
  const { mixnetContractParams } = useContext(AppContext);

  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
    getValues,
    setError,
  } = useForm({ resolver: yupResolver(amountSchema(mixnetContractParams)), defaultValues: amountData });

  const { userBalance } = useContext(AppContext);

  const handleRequestValidation = async (event: { detail: { step: number } }) => {
    let hasSufficientTokens = true;
    const values = getValues();

    if (values.tokenPool === 'balance') {
      hasSufficientTokens = await checkHasEnoughFunds(values.amount.amount);
    }

    if (values.tokenPool === 'locked') {
      hasSufficientTokens = await checkHasEnoughLockedTokens(values.amount.amount);
    }

    if (event.detail.step === 2 && hasSufficientTokens) {
      handleSubmit(onNext)();
    } else {
      setError('amount.amount', { message: 'Not enough tokens' });
    }
  };

  useEffect(() => {
    window.addEventListener('validate_bond_mixnode_step' as any, handleRequestValidation);
    return () => window.removeEventListener('validate_bond_mixnode_step' as any, handleRequestValidation);
  }, []);

  return (
    <Stack gap={3}>
      <Box display="flex" gap={3} justifyContent="center">
        {hasVestingTokens && <TokenPoolSelector disabled={false} onSelect={(pool) => setValue('tokenPool', pool)} />}
        <CurrencyFormField
          required
          fullWidth
          label="Amount"
          autoFocus
          onChanged={(newValue) => {
            setValue('amount', newValue, { shouldValidate: true });
          }}
          validationError={errors.amount?.amount?.message}
          denom={denom}
          initialValue={amountData.amount.amount}
        />
      </Box>
      <Box>
        <CurrencyFormField
          required
          fullWidth
          label="Operating cost"
          onChanged={(newValue) => {
            setValue('operatorCost', newValue, { shouldValidate: true });
          }}
          validationError={errors.operatorCost?.amount?.message}
          denom={denom}
          initialValue={amountData.operatorCost.amount}
        />
        <FormHelperText>
          Monthly operational costs of running your node. If your node is in the active set the amount will be paid back
          to you from the rewards.
        </FormHelperText>
      </Box>
      <Box>
        <TextField
          {...register('profitMargin')}
          name="profitMargin"
          label="Profit margin"
          error={Boolean(errors.profitMargin)}
          helperText={errors.profitMargin?.message}
          fullWidth
        />
        <FormHelperText>
          The percentage of node rewards that you as the node operator take before rewards are distributed to operator
          and delegators.
        </FormHelperText>
      </Box>
      <Box sx={{ mb: 1 }}>
        {!hasVestingTokens && (
          <ModalListItem
            divider
            label="Account balance"
            value={userBalance.balance?.printable_balance.toUpperCase()}
            fontWeight={600}
          />
        )}
        <Typography variant="body2">Est. fee for this transaction will be calculated in the next page</Typography>
      </Box>
    </Stack>
  );
};

export default MixnodeAmountForm;
