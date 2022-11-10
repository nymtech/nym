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
    description="Copy these values in the proxy settings in your application"
    Action={<Button onClick={onClose}>Done</Button>}
  >
    <Box sx={{ mt: 1 }}>
      <Typography fontSize="14px" sx={{ color: 'grey.600' }}>
        Socks5 address
      </Typography>
      <CopyToClipboard text={ipAddress} iconButton light />
      <Typography>{ipAddress}</Typography>
      <Typography fontSize="14px" sx={{ color: 'grey.600', mt: 2 }}>
        Port
      </Typography>
      <Typography>{port}</Typography>
    </Box>
  </InfoModal>
);
