import * as React from 'react';
import { Divider, Stack, Typography } from '@mui/material';
import { DecCoin } from '@nymproject/types';
import { useEffect } from 'react';
import { ErrorOutline } from '@mui/icons-material';
import { SimpleModal } from '../../../../components/Modals/SimpleModal';
import { useBondingContext } from '../../../../context';
import { ConfirmationModal } from '../../../../components';

export interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  onCancel: () => void;
  currentBond: DecCoin;
  addBond: DecCoin;
}

const SummaryModal = ({ open, onClose, onConfirm, onCancel, currentBond, addBond }: Props) => {
  const { getFee, fee, error } = useBondingContext();

  const fetchFee = async () => {
    await getFee('bondMore', {});
  };

  useEffect(() => {
    fetchFee();
  }, []);

  if (error) {
    return (
      <ConfirmationModal
        open={open}
        onClose={onClose}
        onConfirm={onClose}
        title="Operation failed"
        confirmButton="Done"
        maxWidth="xs"
      >
        <Typography variant="caption">Error: {error}</Typography>
        <ErrorOutline color="error" />
      </ConfirmationModal>
    );
  }

  return (
    <SimpleModal
      open={open}
      onClose={onClose}
      onOk={onConfirm}
      onBack={onCancel}
      header="Bond mor details"
      okLabel="Confirm"
    >
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight={400}>Current bond</Typography>
        <Typography fontWeight={400}>{`${currentBond.amount} ${currentBond.denom}`}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight={400}>Additional bond</Typography>
        <Typography fontWeight={400}>{`${addBond.amount} ${addBond.denom}`}</Typography>
      </Stack>
      <Divider sx={{ my: 1 }} />
      <Stack direction="row" justifyContent="space-between">
        <Typography fontWeight={400}>Fee for this operation</Typography>
        <Typography fontWeight={400}>{fee ? `${fee.amount?.amount} ${fee.amount?.denom}` : ''}</Typography>
      </Stack>
    </SimpleModal>
  );
};

export default SummaryModal;
