import React from 'react';
import { Stack, Typography } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { SimpleModal } from '../Modals/SimpleModal';
import { Warning } from '../Warning';

const passwordCreationSteps = [
  'Log out from the wallet',
  'Sign in using “Sign in with mnemonic” button',
  'On the next screen select “Create a password"',
  'Type in the mnemonic you want to create a password for and follow the next steps',
  'Sign back in the wallet using your new password',
  'Come back to this page to import or create new accounts',
];

// TODO add the link href value
export const MultiAccountHowTo = ({ show, handleClose }: { show: boolean; handleClose: () => void }) => (
  <SimpleModal
    open={show}
    onClose={handleClose}
    header="Create account"
    okLabel="Ok"
    onOk={handleClose as () => Promise<void>}
  >
    <Stack spacing={2}>
      <Warning sx={{ textAlign: 'center' }}>
        <Typography fontWeight={600}>
          In order to import or create account(s) you first need to create a password
        </Typography>
      </Warning>
      <Typography fontWeight={600}>How to create a password for your account</Typography>
      {passwordCreationSteps.map((step, index) => (
        <Stack key={step} direction="row" spacing={1}>
          <Typography fontWeight={600}>{`${index + 1}.`}</Typography>
          <Typography>{`${step}`}</Typography>
        </Stack>
      ))}
      <Link
        href="https://nymtech.net/docs/stable/wallet/#importing-or-creating-accounts-when-you-have-signed-in-with-mnemonic"
        target="_blank"
        text="Open Nym docs for this guide in a browser window"
        fontWeight={600}
      />
    </Stack>
  </SimpleModal>
);
