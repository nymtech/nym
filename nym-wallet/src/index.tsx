import React, { useEffect } from 'react';
import ReactDOM from 'react-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { BrowserRouter as Router } from 'react-router-dom';
import { SnackbarProvider } from 'notistack';
import { Routes } from './routes';
import { AppProvider } from './context/main';
import { ErrorFallback } from './components';
import { NymWalletTheme } from './theme';
import { maximizeWindow } from './utils';

const App = () => {
  useEffect(() => {
    maximizeWindow();
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

const root = document.getElementById('root');

ReactDOM.render(<App />, root);
