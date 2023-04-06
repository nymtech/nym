import React from 'react';
import { Stack, Typography } from '@mui/material';
import { ConfirmationModal } from '../../Modals/ConfirmationModal';
import { Alert } from '../../Alert';

export const MultiAccountHowTo = ({ show, handleClose }: { show: boolean; handleClose: () => void }) => (
  <ConfirmationModal
    open={show}
    onClose={handleClose}
    confirmButton="Ok"
    onConfirm={handleClose as () => Promise<void>}
    title=""
    maxWidth="xs"
  >
    <Stack spacing={2}>
      <Alert
        title={
          <Typography sx={{ fontWeight: 600 }}>
            In order to import or create account(s) you first need to create a password
          </Typography>
        }
        bgColor="#fff"
      />
      <Typography>Open security tab in settings to create password to your account</Typography>
      <Typography>
        If you already have password use it to log into wallet and try create/import account again{' '}
      </Typography>
    </Stack>
  </ConfirmationModal>
);
