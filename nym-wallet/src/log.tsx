import React from 'react';
import ReactDOM from 'react-dom';
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

const root = document.getElementById('root-log');

ReactDOM.render(<Log />, root);
