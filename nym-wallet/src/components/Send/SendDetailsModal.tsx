import React from 'react';
import { Stack, SxProps } from '@mui/material';
import { FeeDetails, DecCoin, CurrencyDenom } from '@nymproject/types';
import { SimpleModal } from '../Modals/SimpleModal';
import { ModalListItem } from '../Modals/ModalListItem';
import { ModalFee } from '../Modals/ModalFee';

export const SendDetailsModal = ({
  amount,
  toAddress,
  fromAddress,
  fee,
  denom,
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
  denom: CurrencyDenom;
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
      <ModalListItem label="From:" value={fromAddress} divider />
      <ModalListItem label="To:" value={toAddress} divider />
      <ModalListItem label="Amount:" value={`${amount?.amount} ${denom.toUpperCase()}`} divider />
      <ModalFee fee={fee} divider isLoading={false} />
    </Stack>
  </SimpleModal>
);
