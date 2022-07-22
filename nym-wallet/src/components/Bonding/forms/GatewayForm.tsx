import React, { useEffect } from 'react';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { Box, Checkbox, FormControlLabel, FormHelperText, Stack, TextField } from '@mui/material';
import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { TokenPoolSelector, TPoolOption } from 'src/components';
import { yupResolver } from '@hookform/resolvers/yup';
import { CurrencyDenom } from '@nymproject/types';
import { AmountData, GatewayData, MixnodeData } from 'src/pages/bonding/types';
import { gatewayValidationSchema, amountSchema } from './gatewayValidationSchema';

const NodeData = ({ gatewayData, onNext }: { gatewayData: GatewayData; onNext: (data: GatewayData) => void }) => {
  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
  } = useForm({ resolver: yupResolver(gatewayValidationSchema), defaultValues: gatewayData });

  const captureEvent = (event: { detail: { step: number } }) => {
    if (event.detail.step === 1) {
      handleSubmit(onNext)();
    }
  };

  useEffect(() => {
    window.addEventListener('validate_gateway_step' as any, captureEvent);
    return () => window.removeEventListener('validate_gateway_step' as any, captureEvent);
  }, []);

  return (
    <Stack gap={2}>
      <IdentityKeyFormField
        required
        fullWidth
        label="Identity Key"
        initialValue={gatewayData?.identityKey}
        errorText={errors.identityKey?.message}
        onChanged={(value) => setValue('identityKey', value)}
      />
      <TextField
        {...register('sphinxKey')}
        name="sphinxKey"
        label="Sphinx key"
        error={Boolean(errors.sphinxKey)}
        helperText={errors.sphinxKey?.message}
      />
      <TextField
        {...register('ownerSignature')}
        name="ownerSignature"
        label="Owner signature"
        error={Boolean(errors.ownerSignature)}
        helperText={errors.ownerSignature?.message}
      />
      <TextField
        {...register('location')}
        name="location"
        label="Location"
        error={Boolean(errors.location)}
        helperText={errors.location?.message}
        required
        sx={{ flexBasis: '50%' }}
      />
      <Stack direction="row" gap={2}>
        <TextField
          {...register('host')}
          name="host"
          label="Host"
          error={Boolean(errors.host)}
          helperText={errors.host?.message}
          required
          sx={{ flexBasis: '50%' }}
        />
        <TextField
          {...register('version')}
          name="version"
          label="Version"
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
            label="Mix port"
            error={Boolean(errors.mixPort)}
            helperText={errors.mixPort?.message}
            fullWidth
          />
          <TextField
            {...register('clientsPort')}
            name="clientsPort"
            label="Client WS API port"
            error={Boolean(errors.clientsPort)}
            helperText={errors.clientsPort?.message}
            fullWidth
          />
        </Stack>
      )}
    </Stack>
  );
};

const AmountData = ({
  amountData,
  hasVestingTokens,
  denom,
  onNext,
}: {
  amountData: AmountData;
  hasVestingTokens: boolean;
  denom: CurrencyDenom;
  onNext: (data: any) => void;
}) => {
  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
  } = useForm({ resolver: yupResolver(amountSchema), defaultValues: amountData });

  const captureEvent = (event: { detail: { step: number } }) => {
    console.log('Bond gateway!');
    if (event.detail.step === 2) {
      handleSubmit(onNext)();
    }
  };

  useEffect(() => {
    window.addEventListener('validate_gateway_step' as any, captureEvent);
    return () => window.removeEventListener('validate_gateway_step' as any, captureEvent);
  }, []);

  return (
    <Stack gap={2}>
      <Box display="flex" gap={2} justifyContent="center" sx={{ mt: 2 }}>
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
      </Box>
    </Stack>
  );
};

export const GatewayForm = ({
  step,
  denom,
  gatewayData,
  amountData,
  hasVestingTokens,
  onValidateGatewayData,
  onValidateAmountData,
}: {
  step: 1 | 2;
  gatewayData: GatewayData;
  amountData: AmountData;
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onValidateGatewayData: (data: GatewayData) => void;
  onValidateAmountData: (data: AmountData) => void;
}) => {
  if (step === 1) return <NodeData onNext={onValidateGatewayData} gatewayData={gatewayData} />;

  if (step === 2)
    return (
      <AmountData
        denom={denom}
        amountData={amountData}
        hasVestingTokens={hasVestingTokens}
        onNext={onValidateAmountData}
      />
    );

  return null;
};
