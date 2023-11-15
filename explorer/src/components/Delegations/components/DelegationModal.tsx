import React from 'react';
import { Typography, SxProps, Stack } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { LoadingModal } from './LoadingModal';
import { ConfirmationModal } from './ConfirmationModal';
import { ErrorModal } from './ErrorModal';

export type ActionType = 'delegate' | 'undelegate' | 'redeem' | 'redeem-all' | 'compound';

const actionToHeader = (action: ActionType): string => {
  // eslint-disable-next-line default-case
  switch (action) {
    case 'redeem':
      return 'Rewards redeemed successfully';
    case 'redeem-all':
      return 'All rewards redeemed successfully';
    case 'delegate':
      return 'Delegation successful';
    case 'undelegate':
      return 'Undelegation successful';
    case 'compound':
      return 'Rewards compounded successfully';
    default:
      throw new Error('Unknown type');
  }
};

export type DelegationModalProps = {
  status: 'loading' | 'success' | 'error';
  action: ActionType;
  message?: string;
  transactions?: {
    url: string;
    hash: string;
  }[];
};

export const DelegationModal: FCWithChildren<
  DelegationModalProps & {
    open: boolean;
    onClose: () => void;
    sx?: SxProps;
    backdropProps?: object;
    children?: React.ReactNode;
  }
> = ({ status, action, message, transactions, open, onClose, children, sx, backdropProps }) => {
  if (status === 'loading') return <LoadingModal sx={sx} backdropProps={backdropProps} />;

  if (status === 'error') {
    return (
      <ErrorModal message={message} sx={sx} open={open} onClose={onClose}>
        {children}
      </ErrorModal>
    );
  }

  return (
    <ConfirmationModal
      open={open}
      onConfirm={onClose || (() => {})}
      title={actionToHeader(action)}
      confirmButton="Done"
    >
      <Stack alignItems="center" spacing={2} mb={0}>
        {message && <Typography>{message}</Typography>}
        {transactions?.length === 1 && (
          <Link href={transactions[0].url} target="_blank" sx={{ ml: 1 }} text="View on blockchain" noIcon />
        )}
        {transactions && transactions.length > 1 && (
          <Stack alignItems="center" spacing={1}>
            <Typography>View the transactions on blockchain:</Typography>
            {transactions.map(({ url, hash }) => (
              <Link href={url} target="_blank" sx={{ ml: 1 }} text={hash.slice(0, 6)} key={hash} noIcon />
            ))}
          </Stack>
        )}
      </Stack>
    </ConfirmationModal>
  );
};
