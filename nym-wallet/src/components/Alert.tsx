import React, { useState } from 'react';
import { Alert as MuiAlert, IconButton, SxProps } from '@mui/material';
import { Close } from '@mui/icons-material';

export const Alert = ({
  title,
  dismissable,
  sxAlert,
  bgColor,
}: {
  title: string | React.ReactNode;
  dismissable?: boolean;
  sxAlert?: SxProps;
  bgColor?: string;
}) => {
  const [displayAlert, setDisplayAlert] = useState(true);
  const handleDismiss = () => setDisplayAlert(false);

  if (!displayAlert) return null;

  return (
    <MuiAlert
      severity="info"
      sx={{
        width: '100%',
        borderRadius: 0,
        bgcolor: bgColor || 'background.default',
        color: (theme) => theme.palette.nym.nymWallet.text.blue,
        '& .MuiAlert-icon': { color: 'nym.nymWallet.text.blue', mr: 1 },
        ...sxAlert,
      }}
      action={
        dismissable && (
          <IconButton aria-label="close" color="inherit" size="small" onClick={handleDismiss}>
            <Close fontSize="inherit" />
          </IconButton>
        )
      }
    >
      {title}
    </MuiAlert>
  );
};
