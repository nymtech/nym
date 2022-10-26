import React from 'react';
import { Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { SimpleModal } from '../Modals/SimpleModal';
import { Warning } from '../Warning';

const passwordCreationSteps = [
  'Log out',
  'Click on “Forgot password?” ',
  'On the next screen select “Create new password” ',
  'Create a new password and use it to sign in to your wallet and create multiple accounts',
];

// TODO add the link href value
export const MultiAccountWithPwdHowTo = ({ show, handleClose }: { show: boolean; handleClose: () => void }) => (
  <SimpleModal
    open={show}
    onClose={handleClose}
    header="Create account"
    okLabel="Ok"
    onOk={handleClose as () => Promise<void>}
  >
    <Stack spacing={2}>
      <Warning sx={{ textAlign: 'center' }}>
        <Typography fontWeight={600} sx={{ mb: 1 }}>
          This machine already has a password set on it
        </Typography>
        <Typography>
          In order to import or create account(s) you need to log in with your password or create a new one. Creating a
          new password will overwrite any old one. Make sure your menonics are all wirtten down before creating a new
          password.
        </Typography>
      </Warning>
      <Typography fontWeight={600}>How to create a new password for this account</Typography>
      {passwordCreationSteps.map((step, index) => (
        <Stack key={step} direction="row" spacing={1}>
          <Typography fontWeight={600}>{`${index + 1}.`}</Typography>
          <Typography>{`${step}`}</Typography>
        </Stack>
      ))}
      <Link href="todo" target="_blank" text="Open Nym docs for this guide in a browser window" fontWeight={600} />
    </Stack>
  </SimpleModal>
);
