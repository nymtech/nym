import * as React from 'react';
import { NymNetworkExplorerThemeProvider } from '@nymproject/mui-theme';
import { useMainContext } from '../context/main';

export const NetworkExplorerThemeProvider: React.FC = ({ children }) => {
  const { mode } = useMainContext();
  return <NymNetworkExplorerThemeProvider mode={mode}>{children}</NymNetworkExplorerThemeProvider>;
};
