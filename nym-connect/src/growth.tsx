import React from 'react';
import ReactDOM from 'react-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { ClientContextProvider } from './context/main';
import { ErrorFallback } from './components/Error';
import { NymShipyardTheme } from './theme';
import { TestAndEarnPopup } from './components/Growth/TestAndEarnPopup';
import { TestAndEarnContextProvider } from './components/Growth/context/TestAndEarnContext';

const root = document.getElementById('root-growth');

ReactDOM.render(
  <ErrorBoundary FallbackComponent={ErrorFallback}>
    <ClientContextProvider>
      <TestAndEarnContextProvider>
        <NymShipyardTheme mode="dark">
          <TestAndEarnPopup />
        </NymShipyardTheme>
      </TestAndEarnContextProvider>
    </ClientContextProvider>
  </ErrorBoundary>,
  root,
);
