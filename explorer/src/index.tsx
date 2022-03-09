import * as React from 'react';
import ReactDOM from 'react-dom';
import { BrowserRouter as Router } from 'react-router-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { App } from './App';
import { MainContextProvider } from './context/main';
import './styles.css';
import { NetworkExplorerThemeProvider } from './theme';
import { ErrorBoundaryContent } from './errors/ErrorBoundaryContent';

ReactDOM.render(
  <ErrorBoundary FallbackComponent={ErrorBoundaryContent}>
    <MainContextProvider>
      <NetworkExplorerThemeProvider>
        <Router>
          <App />
        </Router>
      </NetworkExplorerThemeProvider>
    </MainContextProvider>
  </ErrorBoundary>,
  document.getElementById('app'),
);
