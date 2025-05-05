import React from 'react';
import * as Yup from 'yup';
import { Stack, FormControlLabel, Checkbox } from '@mui/material';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { HookFormTextFieldWithPaste } from 'src/components/Clipboard/ClipboardFormFields';
import { useFormContext } from './FormContext';
import { settingsValidationSchema } from './settingsValidationSchema';

type NymNodeDataProps = {
  onClose: () => void;
  // eslint-disable-next-line react/no-unused-prop-types
  onBack: () => void;
  onNext: () => Promise<void>;
  step: number;
};

const validationSchema = Yup.object().shape({
  identity_key: Yup.string().required('Identity key is required'),
  ...settingsValidationSchema.fields,
});

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
    resolver: yupResolver(validationSchema),
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
        {/* Identity Key Field with Paste Button */}
        <HookFormTextFieldWithPaste
          name="identity_key"
          register={register}
          setValue={setValue}
          errors={errors}
          required
          InputLabelProps={{ shrink: true }}
          label="Identity Key"
        />

        {/* Host Field with Built-in Paste */}
        <HookFormTextFieldWithPaste
          name="host"
          label="Host"
          register={register}
          setValue={setValue}
          errors={errors}
          required
          InputLabelProps={{ shrink: true }}
        />

        <FormControlLabel
          control={<Checkbox onChange={() => setShowAdvancedOptions((show) => !show)} checked={showAdvancedOptions} />}
          label="Show advanced options"
        />

        {showAdvancedOptions && (
          <Stack direction="row" gap={3} sx={{ mb: 2 }}>
            <HookFormTextFieldWithPaste
              name="custom_http_port"
              label="Custom HTTP port"
              register={register}
              setValue={setValue}
              errors={errors}
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
