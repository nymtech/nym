import React from 'react';
import { FeeDetails } from '@nymproject/types';
import { CircularProgress } from '@mui/material';
import { ModalListItem } from './ModalListItem';

type TFeeProps = { fee?: FeeDetails; isLoading: boolean; error?: string };

const getValue = ({ fee, isLoading, error }: TFeeProps) => {
  if (isLoading) return <CircularProgress size={15} />;
  if (error && !isLoading) return 'n/a';
  if (fee) return `${fee.amount?.amount} ${fee.amount?.denom}`;
  return '-';
};

export const ModalFee = ({ fee, isLoading, error }: TFeeProps) => (
  <ModalListItem label="Estimated fee for this operation:" value={getValue({ fee, isLoading, error })} />
);
