import React from 'react';
import { Button, Typography } from '@mui/material';
import { Link } from 'react-router-dom';
import MuiLink from '@mui/material/Link';
import { useMainContext } from '@src/context/main';

type NetworkTitleProps = {
  showToggleNetwork?: boolean;
};

const NetworkTitle = ({ showToggleNetwork }: NetworkTitleProps) => {
  const { environment } = useMainContext();

  const explorerName =
    `${environment && environment.charAt(0).toUpperCase() + environment.slice(1)} Explorer` || 'Mainnet Explorer';

  const switchNetworkText = environment === 'mainnet' ? 'Switch to Testnet' : 'Switch to Mainnet';
  const switchNetworkLink =
    environment === 'mainnet' ? 'https://sandbox-explorer.nymtech.net' : 'https://explorer.nymtech.net';
  return (
    <Typography
      variant="h6"
      noWrap
      sx={{
        color: 'nym.networkExplorer.nav.text',
        fontSize: '18px',
        fontWeight: 600,
      }}
    >
      <MuiLink component={Link} to="/overview" underline="none" color="inherit" fontWeight={700}>
        {explorerName}
      </MuiLink>
      {showToggleNetwork && (
        <Button
          variant="outlined"
          color="inherit"
          href={switchNetworkLink}
          sx={{ textTransform: 'none', width: 114, fontSize: '12px', fontWeight: 600, ml: 1 }}
        >
          {switchNetworkText}
        </Button>
      )}
    </Typography>
  );
};

export default NetworkTitle;
