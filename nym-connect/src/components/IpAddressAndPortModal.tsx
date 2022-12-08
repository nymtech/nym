import React from 'react';
import { Box, Button, Typography } from '@mui/material';
import { InfoModal } from './InfoModal';
import { CopyToClipboard } from './CopyToClipboard';

export const IpAddressAndPortModal = ({
  show,
  ipAddress,
  port,
  onClose,
}: {
  show: boolean;
  ipAddress: string;
  port: number;
  onClose: () => void;
}) => (
  <InfoModal
    show={show}
    title="Almost there"
    description="Copy these values to the proxy settings in your application"
    Action={<Button onClick={onClose}>Done</Button>}
  >
    <Box sx={{ mt: 1 }}>
      <Typography fontSize="14px" sx={{ color: 'grey.600' }}>
        Socks5 address
      </Typography>
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <Typography>{ipAddress}</Typography>
        <CopyToClipboard text={ipAddress} iconButton light />
      </Box>

      <Typography fontSize="14px" sx={{ color: 'grey.600', mt: 2 }}>
        Port
      </Typography>
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <Typography>{port}</Typography>
        <CopyToClipboard text={port.toString()} iconButton light />
      </Box>
    </Box>
  </InfoModal>
);
