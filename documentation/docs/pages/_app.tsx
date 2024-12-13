import React, { useMemo } from 'react';
import type { AppProps } from 'next/app';
import './styles.css';
import { ThemeProvider, createTheme } from '@mui/material/styles';

const MyApp: React.FC<AppProps> = ({ Component, pageProps }) => {
  const muiTheme = useMemo(
    () =>
      createTheme({
        palette: {
          mode: 'dark',
          primary: {
            main: '#e67300',
          },
        },
      }),
    [],
  );
  const AnyComponent = Component as any;
  return (
    <ThemeProvider theme={muiTheme}>
      <AnyComponent {...pageProps} />
    </ThemeProvider>
  );
};

export default MyApp;
