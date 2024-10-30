import React from 'react';
import * as yup from 'yup';
import { Stack, TextField, FormControlLabel, Checkbox } from '@mui/material';
import { useForm } from 'react-hook-form';
import { IdentityKeyFormField } from '@nymproject/react/mixnodes/IdentityKeyFormField';
import { yupResolver } from '@hookform/resolvers/yup';
import { isValidHostname, validateRawPort } from 'src/utils';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { useFormContext } from './FormContext';

const yupValidationSchema = yup.object().shape({
  identity_key: yup.string().required('Identity key is required'),
  host: yup
    .string()
    .required('A host is required')
    .test('no-whitespace', 'Host cannot contain whitespace', (value) => !/\s/.test(value || ''))
    .test('valid-host', 'A valid host is required', (value) => (value ? isValidHostname(value) : false)),

  custom_http_port: yup
    .number()
    .nullable()
    .test('valid-http', 'A valid http port is required', (value) => {
      if (value === null) {
        return true;
      }
      return value ? validateRawPort(value) : false;
    }),
});

type NymNodeDataProps = {
  onClose: () => void;
  onBack: () => void;
  onNext: () => Promise<void>;
  step: number;
};

const NymNodeData = ({ onClose, onNext, step }: NymNodeDataProps) => {
  const { setNymNodeData, nymNodeData } = useFormContext();
  const {
    formState: { errors },
    register,
    setValue,
    handleSubmit,
  } = useForm({
    mode: 'all',
    defaultValues: nymNodeData,
    resolver: yupResolver(yupValidationSchema),
  });

  const [showAdvancedOptions, setShowAdvancedOptions] = React.useState(false);

  const handleNext = async () => {
    handleSubmit((args) => {
      setNymNodeData(args);
      onNext();
    })();
  };

  return (
    <SimpleModal
      open
      onOk={handleNext}
      onClose={onClose}
      header="Bond Nym Node"
      subHeader={`Step ${step}/3`}
      okLabel="Next"
      okDisabled={Object.keys(errors).length > 0}
    >
      <Stack gap={3}>
        <IdentityKeyFormField
          autoFocus
          required
          fullWidth
          label="Identity Key"
          initialValue={nymNodeData.identity_key}
          errorText={errors.identity_key?.message?.toString()}
          onChanged={(value) => setValue('identity_key', value, { shouldValidate: true })}
          showTickOnValid={false}
        />

        <TextField
          {...register('host')}
          name="host"
          label="Host"
          error={Boolean(errors.host)}
          helperText={errors.host?.message}
          required
          InputLabelProps={{ shrink: true }}
        />

        <FormControlLabel
          control={<Checkbox onChange={() => setShowAdvancedOptions((show) => !show)} checked={showAdvancedOptions} />}
          label="Show advanced options"
        />
        {showAdvancedOptions && (
          <Stack direction="row" gap={3} sx={{ mb: 2 }}>
            <TextField
              {...register('custom_http_port')}
              name="custom_http_port"
              label="Custom HTTP port"
              error={Boolean(errors.custom_http_port)}
              helperText={errors.custom_http_port?.message}
              fullWidth
              InputLabelProps={{ shrink: true }}
            />
          </Stack>
        )}
      </Stack>
    </SimpleModal>
  );
};

export default NymNodeData;
