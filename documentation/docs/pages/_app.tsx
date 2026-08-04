import React, { useMemo, useEffect } from 'react';
import type { AppProps } from 'next/app';
import dynamic from 'next/dynamic';
import './styles.css';
import './threat-model-viz.css';
import { ThemeProvider, createTheme } from '@mui/material/styles';

// Client-only: the widget uses useChat and streams from /docs/api/chat.
const ChatWidget = dynamic(() => import('components/ChatWidget'), { ssr: false });
// Client-only (ssr:false): portals the per-page Copy/Ask AI buttons into the
// content top. Kept out of SSR/hydration; the theme.config `main` wrapper route
// caused a hydration crash and fixed positioning hid behind the navbar.
const PageActionsMount = dynamic(() => import('components/PageActionsMount'), { ssr: false });

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
      <PageActionsMount />
      <ChatWidget />
    </ThemeProvider>
  );
};

export default MyApp;
