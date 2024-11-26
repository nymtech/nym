import React, { useEffect } from 'react';
import { CurrencyDenom } from '@nymproject/types';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup/dist/yup';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { Box, Stack } from '@mui/material';
import { amountSchema } from './gatewayValidationSchema';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens } from '../../../../utils';
import { GatewayAmount } from '../../../../pages/bonding/types';
import { TokenPoolSelector } from '../../../TokenPoolSelector';

const GatewayAmountForm = ({
  denom,
  amountData,
  hasVestingTokens,
  onNext,
}: {
  denom: CurrencyDenom;
  amountData: GatewayAmount;
  hasVestingTokens: boolean;
  onNext: (data: any) => void;
}) => {
  const {
    formState: { errors },
    handleSubmit,
    setValue,
    getValues,
    setError,
  } = useForm({ resolver: yupResolver(amountSchema), defaultValues: amountData });

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
    window.addEventListener('validate_bond_gateway_step' as any, handleRequestValidation);
    return () => window.removeEventListener('validate_bond_gateway_step' as any, handleRequestValidation);
  }, []);

  return (
    <Stack gap={3}>
      <Box display="flex" gap={3} justifyContent="center" sx={{ mt: 2 }}>
        {hasVestingTokens && <TokenPoolSelector disabled={false} onSelect={(pool) => setValue('tokenPool', pool)} />}
        <CurrencyFormField
          required
          fullWidth
          label="Amount"
          autoFocus
          onChanged={(newValue) => setValue('amount', newValue, { shouldValidate: true })}
          validationError={errors.amount?.amount?.message}
          denom={denom}
          initialValue={amountData.amount.amount}
        />
        <CurrencyFormField
          required
          fullWidth
          label="Operator Cost"
          autoFocus
          onChanged={(newValue) => setValue('operatorCost', newValue, { shouldValidate: true })}
          validationError={errors.operatorCost?.amount?.message}
          denom={denom}
          initialValue={amountData.operatorCost.amount}
        />
      </Box>
    </Stack>
  );
};

export default GatewayAmountForm;
