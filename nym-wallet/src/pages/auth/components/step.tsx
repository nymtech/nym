import React, { useCallback } from 'react';
import { Typography } from '@mui/material';
import { useLocation } from 'react-router-dom';

export const Step = () => {
  const location = useLocation();

  const mapPage = useCallback(() => {
    switch (location.pathname) {
      case '/create-mnemonic':
        return { value: 1, type: 'account', total: 3 };
      case '/verify-mnemonic':
        return { value: 2, type: 'account', total: 3 };
      case '/create-password':
        return { value: 3, type: 'account', total: 3 };
      case '/confirm-mnemonic':
        return { value: 1, type: 'account password', total: 2 };
      case '/connect-password':
        return { value: 2, type: 'account password', total: 2 };
      default:
        return { value: 0, type: '', total: 0 };
    }
  }, [location.pathname]);

  if (mapPage().value === 0) {
    return null;
  }
  const { value, type, total } = mapPage();
  return (
    <Typography
      sx={{ color: (t) => t.palette.nym.nymWallet.text.grey, fontWeight: 400 }}
    >{`Create ${type}. Step ${value}/${total}`}</Typography>
  );
};
