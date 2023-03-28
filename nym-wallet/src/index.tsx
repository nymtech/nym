import React, { useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from 'react-error-boundary';
import { BrowserRouter as Router } from 'react-router-dom';
import { SnackbarProvider } from 'notistack';
import { Routes } from './routes';
import { AppProvider } from './context/main';
import { ErrorFallback } from './components';
import { NymWalletTheme } from './theme';
import { maximizeWindow } from './utils';
import { config } from './config';

const App = () => {
  useEffect(() => {
    // do not maximise in dev mode, because it happens on hot reloading
    if (!config.IS_DEV_MODE) {
      maximizeWindow();
    }
  }, []);

  return (
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <Router>
        <SnackbarProvider
          anchorOrigin={{
            vertical: 'bottom',
            horizontal: 'right',
          }}
        >
          <AppProvider>
            <NymWalletTheme>
              <Routes />
            </NymWalletTheme>
          </AppProvider>
        </SnackbarProvider>
      </Router>
    </ErrorBoundary>
  );
};

const elem = document.getElementById('root');

if (elem) {
  const root = createRoot(elem);
  root.render(<App />);
}
