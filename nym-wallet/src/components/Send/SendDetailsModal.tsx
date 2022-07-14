import React from 'react';
import { Stack } from '@mui/material';
import { SxProps } from '@mui/system';
import { FeeDetails, DecCoin } from '@nymproject/types';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';

export const SendDetailsModal = ({
  amount,
  toAddress,
  fromAddress,
  fee,
  onClose,
  onPrev,
  onSend,
  sx,
  backdropProps,
}: {
  fromAddress?: string;
  toAddress: string;
  fee?: FeeDetails;
  amount?: DecCoin;
  onClose: () => void;
  onPrev: () => void;
  onSend: (data: { val: DecCoin; to: string }) => void;
  sx?: SxProps;
  backdropProps?: object;
}) => (
  <SimpleModal
    header="Send details"
    open
    onClose={onClose}
    okLabel="Confirm"
    onOk={async () => amount && onSend({ val: amount, to: toAddress })}
    onBack={onPrev}
    sx={sx}
    backdropProps={backdropProps}
  >
    <Stack gap={0.5} sx={{ mt: 4 }}>
      <ModalListItem label="From" value={fromAddress} divider />
      <ModalListItem label="To" value={toAddress} divider />
      <ModalListItem label="Amount" value={`${amount?.amount} ${amount?.denom}`} divider />
      <ModalListItem
        label="Fee for this transaction"
        value={!fee ? 'n/a' : `${fee.amount?.amount} ${fee.amount?.denom}`}
        divider
      />
    </Stack>
  </SimpleModal>
);
