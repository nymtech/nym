import React from 'react';
import ReactDOM from 'react-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { LogViewer } from './components/LogViewer';
import { ErrorFallback } from './components/ErrorFallback';
import { NymMixnetTheme } from './theme';

const Log = () => (
  <ErrorBoundary FallbackComponent={ErrorFallback}>
    <NymMixnetTheme mode="dark">
      <LogViewer />
    </NymMixnetTheme>
  </ErrorBoundary>
);

const root = document.getElementById('root-log');

ReactDOM.render(<Log />, root);
