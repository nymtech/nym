import React from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { NymThemeProvider } from './theme/theme';

const rootDOMElem = document.getElementById('root');
if (!rootDOMElem) throw new Error('Root element not found');

const root = createRoot(rootDOMElem);
root.render(
  <NymThemeProvider>
    <App />
  </NymThemeProvider>,
);
