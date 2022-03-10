import React, { useContext } from 'react';
import { Typography } from '@mui/material';
import { Box } from '@mui/system';
import { ClientContext } from '../context/main';
import { CopyToClipboard } from '.';
import { splice } from '../utils';

export const ClientAddress = ({ withCopy }: { withCopy?: boolean }) => {
  const { clientDetails } = useContext(ClientContext);
  return (
    <Box>
      <Typography variant="body2" component="span" sx={{ color: 'grey.600' }}>
        Address:
      </Typography>{' '}
      <Typography variant="body2" component="span" color="nym.background.dark" sx={{ mr: 1 }}>
        {splice(4, 35, clientDetails?.client_address)}
      </Typography>
      {withCopy && <CopyToClipboard text={clientDetails?.client_address} iconButton />}
    </Box>
  );
};
