import React, { useEffect } from 'react';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { Box, Checkbox, FormControlLabel, FormHelperText, Stack, TextField } from '@mui/material';
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { TokenPoolSelector, TPoolOption } from 'src/components';
import { yupResolver } from '@hookform/resolvers/yup';
import { amountSchema, mixnodeValidationSchema } from './mixnodeValidationSchema';
import { CurrencyDenom } from '@nymproject/types';

const NodeData = ({ onNext }: { onNext: (data: any) => void }) => {
  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
  } = useForm({ resolver: yupResolver(mixnodeValidationSchema) });

  const captureEvent = (event: { detail: { step: number } }) => {
    if (event.detail.step === 1) {
      handleSubmit(onNext)();
    }
  };

  useEffect(() => {
    window.addEventListener('validate_step' as any, captureEvent);
    return () => window.removeEventListener('validate_step' as any, captureEvent);
  }, []);

  return (
    <Stack gap={2}>
      <IdentityKeyFormField
        fullWidth
        placeholder="Identity Key"
        required
        onChanged={(value) => setValue('identityKey', value)}
        errorText={errors.identityKey?.message}
      />
      <TextField
        {...register('sphinxKey')}
        name="sphinxKey"
        placeholder="Sphinx key"
        error={Boolean(errors.sphinxKey)}
        helperText={errors.sphinxKey?.message}
      />
      <TextField
        {...register('ownerSignature')}
        name="ownerSignature"
        placeholder="Owner signature"
        error={Boolean(errors.host)}
        helperText={errors.host?.message}
      />
      <Stack direction="row" gap={2}>
        <TextField
          {...register('host')}
          name="host"
          placeholder="Host"
          error={Boolean(errors.host)}
          helperText={errors.host?.message}
          required
          sx={{ flexBasis: '50%' }}
        />
        <TextField
          {...register('version')}
          name="version"
          placeholder="Version"
          error={Boolean(errors.version)}
          helperText={errors.version?.message}
          required
          sx={{ flexBasis: '50%' }}
        />
      </Stack>
      <FormControlLabel
        control={<Checkbox onChange={() => setShowAdvancedOptions((show) => !show)} checked={showAdvancedOptions} />}
        label="Show advanced options"
      />
      {showAdvancedOptions && (
        <Stack direction="row" gap={2} sx={{ mb: 2 }}>
          <TextField
            {...register('mixPort')}
            name="mixPort"
            placeholder="Mix port"
            error={Boolean(errors.mixPort)}
            helperText={errors.mixPort?.message}
            fullWidth
          />
          <TextField
            {...register('verlocPort')}
            name="verloc-port"
            placeholder="Verloc port"
            error={Boolean(errors.verlocPort)}
            helperText={errors.verlocPort?.message}
            fullWidth
          />
          <TextField
            {...register('httpApiPort')}
            name="http-api-port"
            placeholder="HTTP api port"
            error={Boolean(errors.httpApiPort)}
            helperText={errors.httpApiPort?.message}
            fullWidth
          />
        </Stack>
      )}
    </Stack>
  );
};

const AmountData = ({
  hasVestingTokens,
  denom,
  onNext,
}: {
  hasVestingTokens: boolean;
  denom: CurrencyDenom;
  onNext: (data: any) => void;
}) => {
  const [tokenPool, setTokenPool] = useState<TPoolOption>('balance');

  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
  } = useForm({ resolver: yupResolver(amountSchema) });

  const captureEvent = (event: { detail: { step: number } }) => {
    if (event.detail.step === 2) {
      handleSubmit(onNext)();
    }
  };

  useEffect(() => {
    window.addEventListener('validate_step' as any, captureEvent);
    return () => window.removeEventListener('validate_step' as any, captureEvent);
  }, []);

  return (
    <Stack gap={2}>
      <Box display="flex" gap={2} justifyContent="center" sx={{ mt: 2 }}>
        {hasVestingTokens && <TokenPoolSelector disabled={false} onSelect={(pool) => setTokenPool(pool)} />}
        <CurrencyFormField
          required
          fullWidth
          placeholder="Amount"
          autoFocus
          onChanged={(newValue) => setValue('amount', newValue, { shouldValidate: true })}
          validationError={errors.amount?.amount?.message}
          denom={denom}
        />
      </Box>
      <TextField
        {...register('profitMargin')}
        name="profitMargin"
        placeholder="Profit margin"
        error={Boolean(errors.profitMargin)}
        helperText={errors.profitMargin?.message}
      />
    </Stack>
  );
};

export const MixnodeForm = ({
  step,
  denom,
  hasVestingTokens,
  onValidateMixnodeDetail,
  onValidateAmountDetail,
}: {
  step: 1 | 2;
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onValidateMixnodeDetail: (data: any) => void;
  onValidateAmountDetail: (data: any) => void;
}) => {
  if (step === 1) return <NodeData onNext={onValidateMixnodeDetail} />;

  if (step === 2)
    return <AmountData hasVestingTokens={hasVestingTokens} denom={denom} onNext={onValidateAmountDetail} />;

  return null;
};
