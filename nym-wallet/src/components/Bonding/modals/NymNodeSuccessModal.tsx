import React from 'react';
import { Stack, Typography, SxProps, Box, List, ListItem } from '@mui/material';
import { Link } from '@nymproject/react/link/Link';
import { ConfirmationModal } from 'src/components/Modals/ConfirmationModal';
import { ErrorModal } from 'src/components/Modals/ErrorModal';
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline';

export type ConfirmationDetailProps = {
  status: 'success' | 'error';
};

const list = [
  <>
    Node type changed to{' '}
    <Typography component="span" variant="body2" fontWeight="bold">
      Nym node.
    </Typography>
  </>,
  <>
    If you migrated a mix node:{' '}
    <Typography component="span" variant="body2" fontWeight="bold">
      your profit margin and operating cost stayed the same.
    </Typography>
  </>,
  <>
    If you migrated a gateway: by default{' '}
    <Typography component="span" variant="body2" fontWeight="bold">
      your profit margin is set to the minimum of 20% and operating cost to 400 NYM.
    </Typography>
  </>,
  <>
    You can change these values anytime under{' '}
    <Typography component="span" variant="body2" fontWeight="bold">
      node settings.
    </Typography>
  </>,
];

export const NymNodeSuccessModal = ({
  status,
  onClose,
  sx,
  backdropProps,
}: ConfirmationDetailProps & {
  onClose: () => void;
  sx?: SxProps;
  backdropProps?: object;
  children?: React.ReactNode;
}) => {
  if (status === 'error') {
    <ErrorModal open message="error message" onClose={onClose} />;
  }

  return (
    <ConfirmationModal
      open
      onConfirm={onClose}
      onClose={onClose}
      title=""
      confirmButton="Ok"
      maxWidth="xs"
      fullWidth
      sx={sx}
      backdropProps={backdropProps}
    >
      <Box>
        <Box alignItems="center" marginBottom={3}>
          <CheckCircleOutlineIcon sx={{ color: 'success.main', fontSize: 24 }} />
          <Typography variant="subtitle1" fontWeight={600} sx={{ color: 'success.main' }}>
            Sucess!
          </Typography>
          <Typography variant="body2">Your node is now upgraded.</Typography>
        </Box>
        <Stack alignItems="start" spacing={3}>
          <Typography variant="body2" fontWeight={600}>
            List of changes
          </Typography>
          <List sx={{ listStyleType: 'disc', pl: 2 }}>
            {list.map((text, index) => (
              <ListItem key={index} sx={{ display: 'list-item' }}>
                <Typography variant="body2">{text}</Typography>
              </ListItem>
            ))}
          </List>
        </Stack>
      </Box>
    </ConfirmationModal>
  );
};
