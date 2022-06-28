import * as React from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { Divider, Stack, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleDialog, TextFieldInput } from '../../components';
import schema from './schema';

export interface Props {
  open: boolean;
  onClose: () => void;
  onSubmit: (pm: number) => Promise<void>;
  estimatedOpReward: MajorCurrencyAmount;
  currentPm: number;
}

interface FormData {
  profitMargin: number;
}

const NodeSettingsModal = ({ open, onClose, onSubmit, estimatedOpReward, currentPm }: Props) => {
  const {
    control,
    setValue,
    setError,
    handleSubmit,
    formState: { errors },
  } = useForm<FormData>({
    resolver: yupResolver(schema),
    defaultValues: {
      profitMargin: currentPm,
    },
  });

  return (
    <SimpleDialog
      open={open}
      onClose={onClose}
      onConfirm={handleSubmit(async (data) => onSubmit(data.profitMargin))}
      title="Node Settings"
      subTitle="System Variables"
      confirmButton="Next"
      closeButton
    >
      <form>
        <TextFieldInput
          name="profitMargin"
          control={control}
          defaultValue=""
          label="Set profit margin"
          placeholder="Profit Margin"
          error={Boolean(errors.profitMargin)}
          helperText={
            errors.profitMargin
              ? errors.profitMargin.message
              : 'Your new profit margin will be applied in the next epoch'
          }
          required
          muiTextFieldProps={{ fullWidth: true }}
          sx={{ mb: 2.5 }}
        />
      </form>
      <Stack direction="row" justifyContent="space-between" mt={3}>
        <Typography fontWeight={400}>Estimated operator reward for 10% PM</Typography>
        <Typography fontWeight={400}>{`~${estimatedOpReward.amount} ${estimatedOpReward.denom}`}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Typography fontWeight={400}>Est. fee for this transaction will be cauculated in the next page</Typography>
    </SimpleDialog>
  );
};

export default NodeSettingsModal;
