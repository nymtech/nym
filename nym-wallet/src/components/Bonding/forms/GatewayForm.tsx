import React from 'react';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { Checkbox, FormControlLabel, Stack, TextField } from '@mui/material';
import { useState } from 'react';
import { useForm } from 'react-hook-form';

export const GatewayForm = ({ step, hasVestingTokens }: { step: 1 | 2; hasVestingTokens: boolean }) => {
  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const {
    register,
    formState: { errors },
  } = useForm();

  const NodeData = () => (
    <Stack gap={2}>
      <IdentityKeyFormField fullWidth placeholder="Identity Key" required />
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
      <TextField
        {...register('location')}
        name="location"
        placeholder="Location"
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
            {...register('clientApiPort')}
            name="verloc-port"
            placeholder="Verloc port"
            error={Boolean(errors.verlocPort)}
            helperText={errors.verlocPort?.message}
            fullWidth
          />
        </Stack>
      )}
    </Stack>
  );

  const AmountData = () => <h1>Amount data</h1>;

  if (step === 1) return <NodeData />;

  if (step === 2) return <AmountData />;

  return null;
};
