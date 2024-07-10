import { useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { clean } from 'semver';
import { Checkbox, FormControlLabel, Stack, TextField } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react';
import { yupResolver } from '@hookform/resolvers/yup/dist/yup.js';
import { GatewayData } from '../../../pages/bonding/types';
import { gatewayValidationSchema } from './gatewayValidationSchema';

const GatewayInitForm = ({
  gatewayData,
  onNext,
}: {
  gatewayData: GatewayData;
  onNext: (data: GatewayData) => void;
}) => {
  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
  } = useForm({ resolver: yupResolver(gatewayValidationSchema), defaultValues: gatewayData });

  const handleRequestValidation = (event: { detail: { step: number } }) => {
    if (event.detail.step === 1) {
      handleSubmit((data) => {
        const validatedData = {
          ...data,
          version: clean(data.version) as string,
        };
        onNext(validatedData);
      })();
    }
  };

  useEffect(() => {
    window.addEventListener('validate_bond_gateway_step' as any, handleRequestValidation);
    return () => window.removeEventListener('validate_bond_gateway_step' as any, handleRequestValidation);
  }, []);

  return (
    <Stack gap={3}>
      <IdentityKeyFormField
        required
        fullWidth
        label="Identity Key"
        initialValue={gatewayData?.identityKey}
        errorText={errors.identityKey?.message}
        onChanged={(value) => setValue('identityKey', value)}
        showTickOnValid={false}
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
        {...register('location')}
        name="location"
        label="Location"
        error={Boolean(errors.location)}
        helperText={errors.location?.message}
        required
        InputLabelProps={{ shrink: true }}
        sx={{ flexBasis: '50%' }}
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
            {...register('clientsPort')}
            name="clientsPort"
            label="Client WS API port"
            error={Boolean(errors.clientsPort)}
            helperText={errors.clientsPort?.message}
            fullWidth
            InputLabelProps={{ shrink: true }}
          />
        </Stack>
      )}
    </Stack>
  );
};

export default GatewayInitForm;
