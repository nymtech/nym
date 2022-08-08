import React, { useEffect, useState } from 'react';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { Box, Checkbox, FormControlLabel, Stack, TextField } from '@mui/material';
import { useForm } from 'react-hook-form';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { NodeTypeSelector, TokenPoolSelector } from 'src/components';
import { yupResolver } from '@hookform/resolvers/yup';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens } from 'src/utils';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { GatewayAmount, GatewayData } from 'src/pages/bonding/types';
import { gatewayValidationSchema, amountSchema } from './gatewayValidationSchema';

const NodeFormData = ({ gatewayData, onNext }: { gatewayData: GatewayData; onNext: (data: GatewayData) => void }) => {
  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
  } = useForm({ resolver: yupResolver(gatewayValidationSchema), defaultValues: gatewayData });

  const handleRequestValidation = (event: { detail: { step: number } }) => {
    if (event.detail.step === 1) {
      handleSubmit(onNext)();
    }
  };

  useEffect(() => {
    window.addEventListener('validate_bond_gateway_step' as any, handleRequestValidation);
    return () => window.removeEventListener('validate_bond_gateway_step' as any, handleRequestValidation);
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

const AmountFormData = ({
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

export const BondGatewayForm = ({
  step,
  denom,
  gatewayData,
  amountData,
  hasVestingTokens,
  onValidateGatewayData,
  onValidateAmountData,
  onSelectNodeType,
}: {
  step: 1 | 2 | 3;
  gatewayData: GatewayData;
  amountData: GatewayAmount;
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onValidateGatewayData: (data: GatewayData) => void;
  onValidateAmountData: (data: GatewayAmount) => Promise<void>;
  onSelectNodeType: (nodeType: TNodeType) => void;
}) => (
  <>
    {step === 1 && (
      <>
        <Box sx={{ mb: 2 }}>
          <NodeTypeSelector disabled={false} setNodeType={onSelectNodeType} nodeType="gateway" />
        </Box>
        <NodeFormData onNext={onValidateGatewayData} gatewayData={gatewayData} />
      </>
    )}
    {step === 2 && (
      <AmountFormData
        denom={denom}
        amountData={amountData}
        hasVestingTokens={hasVestingTokens}
        onNext={onValidateAmountData}
      />
    )}
  </>
);
