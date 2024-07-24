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
            In order to import or create account(s) you need to log in with password
          </Typography>
        }
      />
      <Typography>
        If you don’t have a password set for your account, log out, click on login with password and follow the forgot
        my password flow
      </Typography>
      <Typography>
        If you already have a password, log in to the wallet using your password then try create/import accounts
      </Typography>
    </Stack>
  </ConfirmationModal>
);
