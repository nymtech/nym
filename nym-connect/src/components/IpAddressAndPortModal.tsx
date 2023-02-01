import React from 'react';
import { Box, Button, Typography } from '@mui/material';
import { InfoModal } from './InfoModal';
import { CopyToClipboard } from './CopyToClipboard';

const FONT_SIZE = '12px';
const FONT_COLOR = 'grey.400';

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
    title="You are half way there"
    description="Check NymConnect menu for supported apps"
    onClose={onClose}
  >
    <Box sx={{ mt: 1 }}>
      <Typography fontSize={FONT_SIZE} color={FONT_COLOR} sx={{ my: 2 }}>
        Paste below values in the proxy settings of your app
      </Typography>
      <Typography fontSize={FONT_SIZE} sx={{ color: 'grey.600' }}>
        Socks5 address
      </Typography>
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <Typography>{ipAddress}</Typography>
        <CopyToClipboard text={ipAddress} iconButton light />
      </Box>

      <Typography fontSize={FONT_SIZE} sx={{ color: 'grey.600', mt: 2 }}>
        Port
      </Typography>
      <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
        <Typography>{port}</Typography>
        <CopyToClipboard text={port.toString()} iconButton light />
      </Box>
    </Box>
  </InfoModal>
);
