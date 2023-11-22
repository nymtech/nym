import * as React from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter as Router } from 'react-router-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { App } from './App';
import { MainContextProvider } from './context/main';
import { NetworkExplorerThemeProvider } from './theme';
import { ErrorBoundaryContent } from './errors/ErrorBoundaryContent';
import CosmosKitProvider from './context/cosmos-kit';
import '@interchain-ui/react/styles';
import './styles.css';

const elem = document.getElementById('app');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <ErrorBoundary FallbackComponent={ErrorBoundaryContent}>
      <MainContextProvider>
        <CosmosKitProvider>
          <NetworkExplorerThemeProvider>
            <Router>
              <App />
            </Router>
          </NetworkExplorerThemeProvider>
        </CosmosKitProvider>
      </MainContextProvider>
    </ErrorBoundary>,
  );
}
