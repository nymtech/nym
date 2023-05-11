import React from 'react';
import { Box, Divider, ListItem, ListItemText, Stack, Typography } from '@mui/material';
import { Button } from 'src/components';
import { PageLayout } from 'src/layouts/PageLayout';
import { useAppContext, useSendContext } from 'src/context';
import { ErrorModal, LoadingModal } from 'src/components/ui/Modal';
import { SendConfirmationModal } from 'src/components/send/SendConfirmationModal';
import { blockExplorerUrl } from 'src/urls';

const InfoItem = ({ label, value }: { label: string; value: string }) => (
  <Box>
    <ListItem disableGutters disablePadding>
      <ListItemText
        primary={
          <Typography fontSize="small" fontWeight={600}>
            {label}
          </Typography>
        }
        secondary={
          <Typography fontSize="small" fontWeight={600}>
            {value}
          </Typography>
        }
      />
    </ListItem>
    <Divider sx={{ my: 1 }} />
  </Box>
);

export const SendConfirmationPage = ({ onCancel }: { onCancel: () => void }) => {
  const { client, denom } = useAppContext();
  const { address, amount, fee, handleSend, transaction, resetTx, onDone } = useSendContext();

  const calculateTotal = () => (Number(fee?.nym) + Number(amount?.amount)).toString();

  return (
    <PageLayout>
      {transaction?.status === 'success' && (
        <SendConfirmationModal
          amount={`${amount?.amount} ${denom}`}
          txUrl={`${blockExplorerUrl}/transactions/${transaction.txHash}`}
          onConfirm={onDone}
        />
      )}
      {transaction?.status === 'loading' && <LoadingModal />}
      {transaction?.status === 'error' && (
        <ErrorModal open title="Transaction failed" onClose={resetTx}>
          <Typography>{transaction.message}</Typography>
        </ErrorModal>
      )}
      <Stack gap={1} height="100%">
        <InfoItem label="From" value={client?.address || ''} />
        <InfoItem label="To" value={address || ''} />
        <InfoItem label="Amount" value={`${amount?.amount} ${denom}`} />
        <InfoItem label="Transaction fee" value={`${fee?.nym || '-'} ${denom}`} />
        <InfoItem label="Total" value={`${calculateTotal()} ${denom}`} />
      </Stack>
      <Box display="flex" gap={2}>
        <Button variant="outlined" size="large" fullWidth onClick={onCancel}>
          Cancel
        </Button>
        <Button variant="contained" size="large" fullWidth onClick={handleSend}>
          Send
        </Button>
      </Box>
    </PageLayout>
  );
};
