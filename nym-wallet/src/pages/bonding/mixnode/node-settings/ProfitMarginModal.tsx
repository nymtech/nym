import * as React from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { Box, Divider, Stack, Tooltip, Typography } from '@mui/material';
import { MajorCurrencyAmount } from '@nymproject/types';
import { SimpleDialog, TextFieldInput } from '../../components';
import { Node as NodeIcon } from '../../../../svg-icons/node';
import getSchema from './schema';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: (pm: number) => void;
  estimatedOpReward: MajorCurrencyAmount;
  currentPm: number;
}

interface FormData {
  profitMargin: number;
}

const NodeSettingsModal = ({ open, onClose, onConfirm, estimatedOpReward, currentPm }: Props) => {
  const {
    control,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<FormData>({
    resolver: yupResolver(getSchema(currentPm)),
    defaultValues: {
      profitMargin: currentPm,
    },
  });

  return (
    <SimpleDialog
      open={open}
      onClose={() => {
        reset();
        onClose();
      }}
      onConfirm={handleSubmit(async (data) => onConfirm(data.profitMargin))}
      title={
        <Stack direction="row" alignItems="center">
          <NodeIcon sx={{ mr: 1, fontSize: 14 }} />
          Node Settings
        </Stack>
      }
      subTitle="System Variables"
      confirmButton="Next"
      closeButton
      disabled={Boolean(errors?.profitMargin)}
    >
      <Box sx={{ mt: 1 }}>
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
          <Tooltip
            title="Estimated total reward in an epoch for this profit margin if your node is selected in the active set."
            arrow
            placement="top"
          >
            <Typography fontWeight={400}>Estimated operator reward for 10% PM</Typography>
          </Tooltip>
          <Typography fontWeight={400}>{`~${estimatedOpReward.amount} ${estimatedOpReward.denom}`}</Typography>
        </Stack>
        <Divider sx={{ my: 1 }} />
        <Typography fontWeight={400}>Est. fee for this transaction will be cauculated in the next page</Typography>
      </Box>
    </SimpleDialog>
  );
};

export default NodeSettingsModal;
