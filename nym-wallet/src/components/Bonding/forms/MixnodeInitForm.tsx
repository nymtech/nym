import React, { useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { clean } from 'semver';
import { Checkbox, FormControlLabel, Stack, TextField } from '@mui/material';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { yupResolver } from '@hookform/resolvers/yup/dist/yup';
import { mixnodeValidationSchema } from './mixnodeValidationSchema';
import { MixnodeData } from '../../../pages/bonding/types';
import { TermsAndConditions, TermsAndConditionsHelp } from './TermsAndConditions';

const MixnodeInitForm = ({ mixnodeData, onNext }: { mixnodeData: MixnodeData; onNext: (data: any) => void }) => {
  const [showAdvancedOptions, setShowAdvancedOptions] = useState(false);

  const {
    register,
    formState: { errors },
    handleSubmit,
    setValue,
    setError,
  } = useForm({ resolver: yupResolver(mixnodeValidationSchema), defaultValues: mixnodeData });

  const handleRequestValidation = (event: { detail: { step: number } }) => {
    if (event.detail.step === 1) {
      handleSubmit((data) => {
        const validatedData = {
          ...data,
          version: clean(data.version),
        };
        if (!validatedData.acceptedTermsAndConditions) {
          setError('acceptedTermsAndConditions', { message: 'You must accept the terms and conditions' });
        } else {
          onNext(validatedData);
        }
      })();
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
      <FormControlLabel
        {...register('acceptedTermsAndConditions')}
        name="acceptedTermsAndConditions"
        required
        control={<Checkbox />}
        label={<TermsAndConditions error={Boolean(errors.acceptedTermsAndConditions)} />}
      />
      <TermsAndConditionsHelp
        error={Boolean(errors.acceptedTermsAndConditions)}
        helperText={errors.acceptedTermsAndConditions?.message}
      />
    </Stack>
  );
};

export default MixnodeInitForm;
