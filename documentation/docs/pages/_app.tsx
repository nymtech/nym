import React, { useMemo, useEffect } from 'react';
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
            main: '#85E89D',
          },
          background: {
            default: '#242B2D',
            paper: '#2A3235',
          },
        },
      }),
    [],
  );
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const img = e.target as HTMLElement;
      if (img.tagName === 'IMG' && img.closest('.nextra-content')) {
        img.classList.toggle('img-expanded');
      }
    };
    document.addEventListener('click', handler);
    return () => document.removeEventListener('click', handler);
  }, []);

  const AnyComponent = Component as any;
  return (
    <ThemeProvider theme={muiTheme}>
      <AnyComponent {...pageProps} />
    </ThemeProvider>
  );
};

export default MyApp;
