import React from 'react';
import { useForm, useWatch } from 'react-hook-form';
import { Stack } from '@mui/material';
import { yupResolver } from '@hookform/resolvers/yup';
import { FieldErrors } from 'react-hook-form/dist/types/errors';
import { GatewayData, MixnodeData, NodeData, NodeType } from '../types';
import { RadioInput, TextFieldInput, CheckboxInput } from '../components';
import nodeSchema from './nodeSchema';
import { SimpleModal } from '../../../components/Modals/SimpleModal';

export interface Props {
  open: boolean;
  onClose?: () => void;
  onSubmit: (data: NodeData) => Promise<void>;
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

const NodeIdentityModal = ({ open, onClose, onSubmit }: Props) => {
  const {
    control,
    getValues,
    handleSubmit,
    formState: { errors },
  } = useForm<NodeData>({
    defaultValues: {
      identityKey: '2UB4668XV7qhmJDPp6KLGWGisiaUYThjA4in2o7WKcwA',
      sphinxKey: '5Rh7X4TwMoUwrQ1ivkqWTCGi1pivmHtenaS7VZDUQPYW',
      signature: '3ccrgwiHhqAbuhhdW7f6UCHZoPFJsQxPcSQRwNc42QVDnDwW8Ebe8p51RhvQp28uqpARysPz52XrE6JuuwJ6fsf8',
      host: '1.1.1.1',
      version: '1.0.7',
      nodeType: 'mixnode',
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
      header="Bond"
      subHeader="Step 1/2"
      okLabel="Next"
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
            error={Boolean((errors as FieldErrors<NodeData<GatewayData>>).location)}
            helperText={(errors as FieldErrors<NodeData<GatewayData>>).location?.message}
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
                  error={Boolean((errors as FieldErrors<NodeData<MixnodeData>>).verlocPort)}
                  helperText={
                    (errors as FieldErrors<NodeData<MixnodeData>>).verlocPort?.message &&
                    'A valid port value is required'
                  }
                  required
                  registerOptions={{ valueAsNumber: true }}
                  sx={{ mb: 2.5 }}
                />
                <TextFieldInput
                  name="httpApiPort"
                  control={control}
                  label="HTTP API Port"
                  placeholder="HTTP API Port"
                  error={Boolean((errors as FieldErrors<NodeData<MixnodeData>>).httpApiPort)}
                  helperText={
                    (errors as FieldErrors<NodeData<MixnodeData>>).httpApiPort?.message &&
                    'A valid port value is required'
                  }
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
                error={Boolean((errors as FieldErrors<NodeData<GatewayData>>).clientsPort)}
                helperText={
                  (errors as FieldErrors<NodeData<GatewayData>>).clientsPort?.message &&
                  'A valid port value is required'
                }
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

export default NodeIdentityModal;
