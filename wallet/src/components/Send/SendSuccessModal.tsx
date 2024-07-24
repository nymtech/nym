import { useContext } from 'react';
import { Stack, Typography, SxProps } from '@mui/material';
import { Link } from '@nymproject/react';
import { AppContext } from 'src/context';
import { TTransactionDetails } from './types';
import { ConfirmationModal } from '../Modals/ConfirmationModal';

export const SendSuccessModal = ({
  txDetails,
  onClose,
  sx,
  backdropProps,
}: {
  txDetails: TTransactionDetails;
  onClose: () => void;
  sx?: SxProps;
  backdropProps?: object;
}) => {
  const { userBalance } = useContext(AppContext);

  const handleClose = async () => {
    await userBalance.refreshBalances();
    onClose();
  };

  return (
    <ConfirmationModal
      open
      onConfirm={handleClose}
      onClose={handleClose}
      title=""
      confirmButton="Done"
      maxWidth="xs"
      fullWidth
      sx={sx}
      backdropProps={backdropProps}
    >
      <Stack alignItems="center" spacing={2}>
        <Typography>You sent</Typography>
        {txDetails && (
          <>
            <Typography variant="h5">{txDetails.amount}</Typography>
            <Link href={txDetails.txUrl} target="_blank" sx={{ ml: 1 }} text="View on blockchain" />
          </>
        )}
      </Stack>
    </ConfirmationModal>
  );
};
