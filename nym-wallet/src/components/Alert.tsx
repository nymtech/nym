import React, { useState } from 'react';
import { Alert as MuiAlert, IconButton } from '@mui/material';
import { Close } from '@mui/icons-material';

export const Alert = ({ title, dismissable }: { title: string | React.ReactNode; dismissable?: boolean }) => {
  const [displayAlert, setDisplayAlert] = useState(true);
  const handleDismiss = () => setDisplayAlert(false);

  if (!displayAlert) return null;

  return (
    <MuiAlert
      severity="info"
      sx={{
        width: '100%',
        borderRadius: 0,
        bgcolor: 'background.default',
        color: (theme) => theme.palette.nym.nymWallet.text.blue,
        '& .MuiAlert-icon': { color: 'nym.nymWallet.text.blue', mr: 1 },
      }}
      action={
        <IconButton aria-label="close" color="inherit" size="small" onClick={dismissable ? handleDismiss : undefined}>
          <Close fontSize="inherit" />
        </IconButton>
      }
    >
      {title}
    </MuiAlert>
  );
};
