import React, { useContext, useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { Box, Checkbox, FormControlLabel, FormHelperText, Stack, TextField, Typography } from '@mui/material';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { CurrencyDenom, TNodeType } from '@nymproject/types';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens } from 'src/utils';
import { NodeTypeSelector, TokenPoolSelector } from 'src/components';
import { MixnodeAmount, MixnodeData } from 'src/pages/bonding/types';
import { ModalListItem } from 'src/components/Modals/ModalListItem';
import { AppContext } from 'src/context';
import { amountSchema, mixnodeValidationSchema } from './mixnodeValidationSchema';

const NodeFormData = ({ mixnodeData, onNext }: { mixnodeData: MixnodeData; onNext: (data: any) => void }) => {
  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
  } = useForm({ resolver: yupResolver(mixnodeValidationSchema), defaultValues: mixnodeData });

  const handleRequestValidation = (event: { detail: { step: number } }) => {
    if (event.detail.step === 1) {
      handleSubmit(onNext)();
    }
  };

  useEffect(() => {
    window.addEventListener('validate_bond_mixnode_step' as any, handleRequestValidation);
    return () => window.removeEventListener('validate_bond_mixnode_step' as any, handleRequestValidation);
  }, []);

  return (
    <Stack gap={3}>
      <IdentityKeyFormField
        required
        fullWidth
        label="Identity Key"
        initialValue={mixnodeData?.identityKey}
        errorText={errors.identityKey?.message}
        onChanged={(value) => setValue('identityKey', value)}
      />
      <TextField
        {...register('sphinxKey')}
        name="sphinxKey"
        label="Sphinx key"
        error={Boolean(errors.sphinxKey)}
        helperText={errors.sphinxKey?.message}
        InputLabelProps={{ shrink: true }}
      />
      <TextField
        {...register('ownerSignature')}
        name="ownerSignature"
        label="Owner signature"
        error={Boolean(errors.ownerSignature)}
        helperText={errors.ownerSignature?.message}
        InputLabelProps={{ shrink: true }}
      />
      <Stack direction="row" gap={3}>
        <TextField
          {...register('host')}
          name="host"
          label="Host"
          error={Boolean(errors.host)}
          helperText={errors.host?.message}
          required
          InputLabelProps={{ shrink: true }}
          sx={{ flexBasis: '50%' }}
        />
        <TextField
          {...register('version')}
          name="version"
          label="Version"
          error={Boolean(errors.version)}
          helperText={errors.version?.message}
          required
          InputLabelProps={{ shrink: true }}
          sx={{ flexBasis: '50%' }}
        />
      </Stack>
      <FormControlLabel
        control={<Checkbox onChange={() => setShowAdvancedOptions((show) => !show)} checked={showAdvancedOptions} />}
        label="Show advanced options"
      />
      {showAdvancedOptions && (
        <Stack direction="row" gap={3} sx={{ mb: 2 }}>
          <TextField
            {...register('mixPort')}
            name="mixPort"
            label="Mix port"
            error={Boolean(errors.mixPort)}
            helperText={errors.mixPort?.message}
            fullWidth
            InputLabelProps={{ shrink: true }}
          />
          <TextField
            {...register('verlocPort')}
            name="verlocPort"
            label="Verloc port"
            error={Boolean(errors.verlocPort)}
            helperText={errors.verlocPort?.message}
            fullWidth
            InputLabelProps={{ shrink: true }}
          />
          <TextField
            {...register('httpApiPort')}
            name="httpApiPort"
            label="HTTP api port"
            error={Boolean(errors.httpApiPort)}
            helperText={errors.httpApiPort?.message}
            fullWidth
            InputLabelProps={{ shrink: true }}
          />
        </Stack>
      )}
    </Stack>
  );
};

const AmountFormData = ({
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
  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
    getValues,
    setError,
  } = useForm({ resolver: yupResolver(amountSchema), defaultValues: amountData });

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

export const BondMixnodeForm = ({
  step,
  denom,
  mixnodeData,
  amountData,
  hasVestingTokens,
  onValidateMixnodeData,
  onValidateAmountData,
  onSelectNodeType,
}: {
  step: 1 | 2 | 3;
  mixnodeData: MixnodeData;
  amountData: MixnodeAmount;
  denom: CurrencyDenom;
  hasVestingTokens: boolean;
  onValidateMixnodeData: (data: MixnodeData) => void;
  onValidateAmountData: (data: MixnodeAmount) => Promise<void>;
  onSelectNodeType: (nodeType: TNodeType) => void;
}) => (
  <>
    {step === 1 && (
      <>
        <Box sx={{ mb: 3 }}>
          <NodeTypeSelector disabled={false} setNodeType={onSelectNodeType} nodeType="mixnode" />
        </Box>
        <NodeFormData onNext={onValidateMixnodeData} mixnodeData={mixnodeData} />
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
