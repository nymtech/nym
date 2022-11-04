import React from 'react';
import ReactDOM from 'react-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { ClientContextProvider } from './context/main';
import { ErrorFallback } from './components/Error';
import { NymMixnetTheme } from './theme';
import { App } from './App';
import { AppWindowFrame } from './components/AppWindowFrame';
import './fonts/fonts.css';

const root = document.getElementById('root');

ReactDOM.render(
  <ErrorBoundary FallbackComponent={ErrorFallback}>
    <ClientContextProvider>
      <NymMixnetTheme>
        <AppWindowFrame>
          <App />
        </AppWindowFrame>
      </NymMixnetTheme>
    </ClientContextProvider>
  </ErrorBoundary>,
  root,
);
