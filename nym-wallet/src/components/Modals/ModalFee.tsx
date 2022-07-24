import React from 'react';
import { FeeDetails } from '@nymproject/types';
import { CircularProgress } from '@mui/material';
import { ModalListItem } from './ModalListItem';
import { ModalDivider } from './ModalDivider';

type TFeeProps = { fee?: FeeDetails; isLoading: boolean; error?: string; divider?: boolean };

const getValue = ({ fee, isLoading, error }: TFeeProps) => {
  if (isLoading) return <CircularProgress size={15} />;
  if (error && !isLoading) return 'n/a';
  if (fee) return `${fee.amount?.amount} ${fee.amount?.denom}`;
  return '-';
};

<<<<<<< HEAD
export const ModalFee = ({ fee, isLoading, error }: TFeeProps) => (
  <ModalListItem label="Estimated fee for this operation:" value={getValue({ fee, isLoading, error })} />
=======
export const ModalFee = ({ fee, isLoading, error, divider }: TFeeProps) => (
  <>
    <ModalListItem label="Fee for this operation" value={getValue({ fee, isLoading, error })} />
    {divider && <ModalDivider />}
  </>
>>>>>>> 1685352c8 (check balance and fetch fee on bond mixnode request)
);
