import React from 'react';
import ReactDOM from 'react-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { ClientContextProvider } from './context/main';
import { ErrorFallback } from './components/Error';
import { NymMixnetTheme } from './theme';
import './fonts/fonts.css';
import { App } from './App';

const root = document.getElementById('root');

ReactDOM.render(
  <ErrorBoundary FallbackComponent={ErrorFallback}>
    <ClientContextProvider>
      <NymMixnetTheme>
        <App />
      </NymMixnetTheme>
    </ClientContextProvider>
  </ErrorBoundary>,
  root,
);
