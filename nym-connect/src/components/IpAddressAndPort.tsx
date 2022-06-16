import { Box, Tooltip, Typography } from '@mui/material';
import React from 'react';
import { styled } from '@mui/system';

const IpAddressAndPortContainer = styled('div')({
  '.hoverAddressCopy:hover': {
    cursor: 'pointer',
    textDecoration: 'underline',
    textDecorationColor: '#FB6E4E',
    textDecorationThickness: '2px',
    textUnderlineOffset: '4px',
  },
});

export const IpAddressAndPort: React.FC<{
  label: string;
  ipAddress: string;
  port: number;
}> = ({ label, ipAddress, port }) => (
  <IpAddressAndPortContainer>
    <Box display="flex" justifyContent="space-between" color="rgba(255,255,255,0.6)">
      <Typography fontSize="14px">{label}</Typography>
      <Typography fontSize="14px">Port</Typography>
    </Box>
    <Box display="flex" justifyContent="space-between">
      <Tooltip title="Click to copy SOCKS5 proxy hostname">
        <Typography fontWeight="600" className="hoverAddressCopy">
          {ipAddress}
        </Typography>
      </Tooltip>
      <Tooltip title="Click to copy SOCKS5 proxy port">
        <Typography fontWeight="600" className="hoverAddressCopy">
          {port}
        </Typography>
      </Tooltip>
    </Box>
  </IpAddressAndPortContainer>
);
