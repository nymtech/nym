import { Box, Stack, Tooltip, Typography } from '@mui/material';
import React from 'react';
import { styled } from '@mui/system';
import { writeText } from '@tauri-apps/api/clipboard';
import CheckCircleOutlineIcon from '@mui/icons-material/CheckCircleOutline';

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
        <Typography fontSize="14px">{label}</Typography>
        <Typography fontSize="14px">Port</Typography>
      </Box>
      <Box display="flex" justifyContent="space-between">
        <Tooltip
          title={
            ipAddressCopied ? (
              <Stack direction="row" spacing={1} fontSize="inherit" alignItems="center">
                <CheckCircleOutlineIcon color="success" fontSize="small" />
                <Typography fontSize="inherit">SOCKS5 proxy hostname copied to the clipboard</Typography>
              </Stack>
            ) : (
              <>Click to copy SOCKS5 proxy hostname</>
            )
          }
        >
          <Typography
            fontWeight="600"
            className="hoverAddressCopy"
            onClick={async () => {
              await writeText(`${ipAddress}`);
              setIpAddressCopied(true);
            }}
          >
            {ipAddress}
          </Typography>
        </Tooltip>
        <Tooltip
          title={
            portCopied ? (
              <Stack direction="row" spacing={1} fontSize="inherit" alignItems="center">
                <CheckCircleOutlineIcon color="success" fontSize="small" />
                <Typography fontSize="inherit">SOCKS5 proxy port copied to the clipboard</Typography>
              </Stack>
            ) : (
              <>Click to copy SOCKS5 proxy port</>
            )
          }
        >
          <Typography
            fontWeight="600"
            className="hoverAddressCopy"
            onClick={async () => {
              await writeText(`${port}`);
              setPortCopied(true);
            }}
          >
            {port}
          </Typography>
        </Tooltip>
      </Box>
    </IpAddressAndPortContainer>
  );
};
