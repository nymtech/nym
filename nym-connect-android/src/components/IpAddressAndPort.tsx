import { Box, Typography } from '@mui/material';
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

export const IpAddressAndPort: FCWithChildren<{
  label: string;
  ipAddress: string;
  port: number;
}> = ({ label, ipAddress, port }) => {
  const [ipAddressCopied, setIpAddressCopied] = React.useState<boolean>(false);
  const [portCopied, setPortCopied] = React.useState<boolean>(false);

  React.useEffect(() => {
    if (ipAddressCopied) {
      setTimeout(() => setIpAddressCopied(false), 2000);
    }
  }, [ipAddressCopied]);

  React.useEffect(() => {
    if (portCopied) {
      setTimeout(() => setPortCopied(false), 2000);
    }
  }, [portCopied]);

  return (
    <IpAddressAndPortContainer>
      <Box display="flex" justifyContent="space-between" color="rgba(255,255,255,0.6)">
        <Typography fontSize="14px" sx={{ color: 'grey.600' }} fontWeight={400}>
          {label}
        </Typography>
        <Typography fontSize="14px" sx={{ color: 'grey.600' }} fontWeight={400}>
          Port
        </Typography>
      </Box>
      <Box display="flex" justifyContent="space-between">
        <Typography fontWeight="400" className="hoverAddressCopy" fontSize="20px">
          {ipAddress}
        </Typography>
        <Typography fontWeight="400" fontSize="20px" className="hoverAddressCopy">
          {port}
        </Typography>
      </Box>
    </IpAddressAndPortContainer>
  );
};
