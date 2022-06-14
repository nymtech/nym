import React from 'react';
import { useForm, useWatch } from 'react-hook-form';
import { Stack } from '@mui/material';
import { yupResolver } from '@hookform/resolvers/yup';
import { SimpleModal } from '../../components/Modals/SimpleModal';
import { NodeData, NodeType } from './types';
import { RadioInput, TextFieldInput, CheckboxInput } from './bond-form';
import { nodeSchema } from './nodeSchema';

export interface Props {
  open: boolean;
  onClose?: () => void;
  onSubmit: (data: NodeData) => Promise<void>;
  header?: string;
  buttonText?: string;
}

const radioOptions: { label: string; value: NodeType }[] = [
  {
    label: 'Mixnode',
    value: 'mixnode',
  },
  {
    label: 'Gateway',
    value: 'gateway',
  },
];

export const NodeIdentityModal = ({ open, onClose, onSubmit, header, buttonText }: Props) => {
  const {
    control,
    getValues,
    handleSubmit,
    formState: { errors },
  } = useForm<NodeData>({
    defaultValues: {
      nodeType: radioOptions[0].value,
      advancedOpt: false,
      mixPort: 1789,
      verlocPort: 1790,
      httpApiPort: 8000,
      clientsPort: 9000,
    },
    resolver: yupResolver(nodeSchema),
  });

  const nodeType = useWatch({ name: 'nodeType', control });
  const advancedOpt = useWatch({ name: 'advancedOpt', control });

  const onSubmitForm = (data: NodeData) => {
    onSubmit(data);
  };

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={handleSubmit(onSubmitForm)}
      header={header || 'Bond'}
      subHeader="Step 1/2"
      okLabel={buttonText || 'Next'}
    >
      <form>
        <RadioInput
          name="nodeType"
          label="Select node type"
          options={radioOptions}
          control={control}
          defaultValue={getValues('nodeType')}
          muiRadioGroupProps={{ row: true }}
        />
        <TextFieldInput
          name="identityKey"
          control={control}
          defaultValue=""
          label="Identity Key"
          placeholder="Identity Key"
          error={Boolean(errors.identityKey)}
          helperText={errors.identityKey?.message}
          required
          muiTextFieldProps={{ fullWidth: true }}
          sx={{ mb: 2.5, mt: 1 }}
        />
        <TextFieldInput
          name="sphinxKey"
          control={control}
          defaultValue=""
          label="Sphinx Key"
          placeholder="Sphinx Key"
          error={Boolean(errors.sphinxKey)}
          helperText={errors.sphinxKey?.message}
          required
          muiTextFieldProps={{ fullWidth: true }}
          sx={{ mb: 2.5 }}
        />
        <TextFieldInput
          name="signature"
          control={control}
          defaultValue=""
          label="Signature"
          placeholder="Signature"
          error={Boolean(errors.signature)}
          helperText={errors.signature?.message}
          required
          muiTextFieldProps={{ fullWidth: true }}
          sx={{ mb: 2.5 }}
        />
        {nodeType === 'gateway' && (
          <TextFieldInput
            name="location"
            control={control}
            defaultValue=""
            label="Location"
            placeholder="Location"
            error={Boolean(errors.location)}
            helperText={errors.location?.message}
            required
            muiTextFieldProps={{ fullWidth: true }}
            sx={{ mb: 2.5 }}
          />
        )}
        <Stack direction="row" spacing={2}>
          <TextFieldInput
            name="host"
            control={control}
            defaultValue=""
            label="Host"
            placeholder="Host"
            error={Boolean(errors.host)}
            helperText={errors.host?.message}
            required
            muiTextFieldProps={{ fullWidth: true }}
            sx={{ mb: 2.5 }}
          />
          <TextFieldInput
            name="version"
            control={control}
            defaultValue=""
            label="Version"
            placeholder="Version"
            error={Boolean(errors.version)}
            helperText={errors.version?.message}
            required
            muiTextFieldProps={{ fullWidth: true }}
            sx={{ mb: 2.5 }}
          />
        </Stack>
        <CheckboxInput
          name="advancedOpt"
          label="Use advanced options"
          control={control}
          defaultValue={false}
          sx={{ mb: 2.5 }}
        />
        {advancedOpt && (
          <Stack direction="row" spacing={1.5}>
            <TextFieldInput
              name="mixPort"
              control={control}
              label="Mix Port"
              placeholder="Mix Port"
              error={Boolean(errors.mixPort)}
              helperText={errors.mixPort?.message && 'A valid port value is required'}
              required
              registerOptions={{ valueAsNumber: true }}
              sx={{ mb: 2.5 }}
            />
            {nodeType === 'mixnode' ? (
              <>
                <TextFieldInput
                  name="verlocPort"
                  control={control}
                  label="Verloc Port"
                  placeholder="Verloc Port"
                  error={Boolean(errors.verlocPort)}
                  helperText={errors.verlocPort?.message && 'A valid port value is required'}
                  required
                  registerOptions={{ valueAsNumber: true }}
                  sx={{ mb: 2.5 }}
                />
                <TextFieldInput
                  name="httpApiPort"
                  control={control}
                  label="HTTP API Port"
                  placeholder="HTTP API Port"
                  error={Boolean(errors.httpApiPort)}
                  helperText={errors.httpApiPort?.message && 'A valid port value is required'}
                  required
                  registerOptions={{ valueAsNumber: true }}
                  sx={{ mb: 2.5 }}
                />
              </>
            ) : (
              <TextFieldInput
                name="clientsPort"
                control={control}
                label="client WS API Port"
                placeholder="client WS API Port"
                error={Boolean(errors.clientsPort)}
                helperText={errors.clientsPort?.message && 'A valid port value is required'}
                required
                registerOptions={{ valueAsNumber: true }}
                sx={{ mb: 2.5 }}
              />
            )}
          </Stack>
        )}
      </form>
    </SimpleModal>
  );
};
