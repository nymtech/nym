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
        <Typography fontWeight={600} fontSize={14} sx={{ mb: 1 }}>
          This machine already has a password set on it
        </Typography>
        <Typography fontSize={12}>
          In order to import or create account(s) you need to log in with your password or create a new one. Creating a
          new password will overwrite any old one. Make sure your menonics are all wirtten down before creating a new
          password.
        </Typography>
      </Warning>
      <Typography fontWeight={600}>How to create a new password for this account</Typography>
      {passwordCreationSteps.map((step, index) => (
        <Stack key={step} direction="row" spacing={1}>
          <Typography fontWeight={600}>{`${index + 1}.`}</Typography>
          <Typography fontSize={14}>{`${step}`}</Typography>
        </Stack>
      ))}
      <Link
        href="https://nymtech.net/docs/stable/wallet#importing-or-creating-accounts-when-you-have-signed-in-with-mnemonic-but-a-password-already-exists-on-your-machine"
        target="_blank"
        text="Open Nym docs for this guide in a browser window"
        fontWeight={600}
        fontSize={14}
      />
    </Stack>
  </SimpleModal>
);
