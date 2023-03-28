import * as React from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter as Router } from 'react-router-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { App } from './App';
import { MainContextProvider } from './context/main';
import './styles.css';
import { NetworkExplorerThemeProvider } from './theme';
import { ErrorBoundaryContent } from './errors/ErrorBoundaryContent';

const elem = document.getElementById('app');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <ErrorBoundary FallbackComponent={ErrorBoundaryContent}>
      <MainContextProvider>
        <NetworkExplorerThemeProvider>
          <Router>
            <App />
          </Router>
        </NetworkExplorerThemeProvider>
      </MainContextProvider>
    </ErrorBoundary>,
  );
}
