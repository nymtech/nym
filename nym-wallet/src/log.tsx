import React from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from 'react-error-boundary';
import { LogViewer } from './components/LogViewer';
import { ErrorFallback } from './components';
import { NymWalletTheme } from './theme';

const Log = () => (
  <ErrorBoundary FallbackComponent={ErrorFallback}>
    <NymWalletTheme>
      <LogViewer />
    </NymWalletTheme>
  </ErrorBoundary>
);

const elem = document.getElementById('root-log');

if (elem) {
  const root = createRoot(elem);
  root.render(<Log />);
}
