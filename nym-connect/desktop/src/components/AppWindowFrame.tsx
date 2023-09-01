import React from 'react';
import { Box } from '@mui/material';
import { useLocation } from 'react-router-dom';
import { useClientContext } from 'src/context/main';
import { CustomTitleBar } from './CustomTitleBar';

export const AppWindowFrame: FCWithChildren = ({ children }) => {
  const location = useLocation();
  const { userDefinedGateway, setUserDefinedGateway, userDefinedSPAddress, setUserDefinedSPAddress } =
    useClientContext();

  // defined functions to be used when moving away from pages
  const onBack = () => {
    switch (location.pathname) {
      case '/menu/settings/gateway':
        return () => {
          // when the user moves away from the settings page and the gateway is not valid
          // set isActive to false
          if (!userDefinedGateway?.address) {
            setUserDefinedGateway((current) => ({ ...current, isActive: false }));
          }
        };
      case '/menu/settings/service-provider':
        return () => {
          // when the user moves away from the settings page and the sp is not valid
          // set isActive to false
          if (!userDefinedSPAddress?.address) {
            setUserDefinedSPAddress((current) => ({ ...current, isActive: false }));
          }
        };
      default:
        return undefined;
    }
  };

  return (
    <Box
      sx={{
        display: 'grid',
        gridTemplateRows: '40px 1fr',
        height: '100vh',
      }}
    >
      <CustomTitleBar path={location.pathname} onBack={onBack()} />
      <Box style={{ padding: '16px' }}>{children}</Box>
    </Box>
  );
};
