import React from 'react';
import { Box, Button, CircularProgress, Link, Modal, Stack, Typography } from '@mui/material';
import { modalStyle } from '../Modals/styles';

export type ActionType = 'delegate' | 'undelegate' | 'redeem' | 'redeem-all';

const actionToHeader = (action: ActionType): string => {
  // eslint-disable-next-line default-case
  switch (action) {
    case 'redeem':
      return 'Rewards redeemed successfully';
    case 'redeem-all':
      return 'All rewards redeemed successfully';
    case 'delegate':
      return 'Delegation complete';
    case 'undelegate':
      return 'Undelegation complete';
  }
  return 'Oh no! Something went wrong!';
};

export type DelegationModalProps = {
  status: 'loading' | 'success' | 'error';
  action: ActionType;
  message?: string;
  recipient?: string;
  balance?: string;
  transactionUrl?: string;
};

export const DelegationModal: React.FC<
  DelegationModalProps & {
    open: boolean;
    onClose?: () => void;
  }
> = ({ status, action, message, recipient, balance, transactionUrl, open, onClose, children }) => {
  if (status === 'loading') {
    return (
      <Modal open>
        <Box sx={modalStyle} textAlign="center">
          <Stack spacing={4} direction="row" alignItems="center">
            <CircularProgress />
            <Typography>Please wait...</Typography>
          </Stack>
        </Box>
      </Modal>
    );
  }

  if (status === 'error') {
    return (
      <Modal open={open} onClose={onClose}>
        <Box sx={modalStyle} textAlign="center">
          <Typography color={(theme) => theme.palette.error.main} mb={1}>
            Oh no! Something went wrong...
          </Typography>
          <Typography my={5}>{message}</Typography>
          {children}
          <Button variant="contained" onClick={onClose}>
            Close
          </Button>
        </Box>
      </Modal>
    );
  }
  return (
    <Modal open={open} onClose={onClose}>
      <Box sx={modalStyle} textAlign="center">
        <Typography color={(theme) => theme.palette.success.main} mb={1}>
          {actionToHeader(action)}
        </Typography>
        <Typography mb={3}>{message}</Typography>

        {recipient && (
          <Typography mb={1} fontSize="small" color={(theme) => theme.palette.text.secondary}>
            Recipient: {recipient}
          </Typography>
        )}
        <Typography mb={1} fontSize="small" color={(theme) => theme.palette.text.secondary}>
          Your current balance: {balance}
        </Typography>
        <Typography mb={1} fontSize="small" color={(theme) => theme.palette.text.secondary}>
          Check the transaction hash{' '}
          <Link href={transactionUrl} target="_blank">
            here
          </Link>
        </Typography>
        {children}
        <Button variant="contained" sx={{ mt: 3 }} size="large" onClick={onClose}>
          Finish
        </Button>
      </Box>
    </Modal>
  );
};
