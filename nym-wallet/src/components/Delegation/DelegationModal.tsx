import React from 'react';
import { Box, Button, CircularProgress, Modal, Stack, Typography } from '@mui/material';
import { modalStyle } from '../Modals/styles';
import { TPoolOption } from '../TokenPoolSelector';
import { Link } from '../Link';

export type ActionType = 'delegate' | 'undelegate' | 'redeem' | 'redeem-all' | 'compound';

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
    case 'compound':
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
  tokenPool?: TPoolOption;
};

export const DelegationModal: React.FC<
  DelegationModalProps & {
    open: boolean;
    onClose?: () => void;
  }
> = ({ status, action, message, recipient, balance, transactionUrl, open, onClose, tokenPool, children }) => {
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
          Your current {tokenPool === 'locked' ? 'locked balance' : 'balance'}: {balance}
        </Typography>
        <Typography mb={1} fontSize="small" color={(theme) => theme.palette.text.secondary}>
          Check the transaction hash <Link href={transactionUrl} target="_blank" text="here" />
        </Typography>
        {children}
        <Button variant="contained" sx={{ mt: 3 }} size="large" onClick={onClose}>
          Finish
        </Button>
      </Box>
    </Modal>
  );
};
