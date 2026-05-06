import React from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from 'react-error-boundary';
import { SnackbarProvider } from 'notistack';
import { LogViewer } from './components/LogViewer';
import { ErrorFallback } from './components';
import { NymWalletTheme } from './theme';

const Log = () => (
  <ErrorBoundary FallbackComponent={ErrorFallback}>
    <SnackbarProvider anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}>
      <NymWalletTheme>
        <LogViewer />
      </NymWalletTheme>
    </SnackbarProvider>
  </ErrorBoundary>
);

const elem = document.getElementById('root-log');

if (elem) {
  const root = createRoot(elem);
  root.render(<Log />);
}
