import { FeeDetails } from '@nymproject/types';
import { CircularProgress } from '@mui/material';
import { ModalListItem } from './ModalListItem';
import { ModalDivider } from './ModalDivider';

type TFeeProps = { fee?: FeeDetails; isLoading: boolean; error?: string; divider?: boolean };
type TTotalAmountProps = { fee?: FeeDetails; amount?: string; isLoading: boolean; error?: string; divider?: boolean };

const getValue = ({ fee, amount, isLoading, error }: TTotalAmountProps) => {
  if (isLoading) return <CircularProgress size={15} />;
  if (error && !isLoading) return 'n/a';
  if (fee) {
    const numericFee = Number(fee.amount?.amount);
    const numericAmountToTransfer = Number(amount);
    return amount
      ? `${numericFee + numericAmountToTransfer}  ${fee.amount?.denom.toUpperCase()}`
      : `${fee.amount?.amount} ${fee.amount?.denom.toUpperCase()}`;
  }
  return '-';
};

export const ModalFee = ({ fee, isLoading, error, divider }: TFeeProps) => (
  <>
    <ModalListItem label="Fee for this transaction" value={getValue({ fee, isLoading, error })} />
    {divider && <ModalDivider />}
  </>
);

export const ModalTotalAmount = ({ fee, amount, isLoading, error, divider }: TTotalAmountProps) => (
  <>
    <ModalListItem label="Total amount" value={getValue({ fee, amount, isLoading, error })} fontWeight={600} />
    {divider && <ModalDivider />}
  </>
);
