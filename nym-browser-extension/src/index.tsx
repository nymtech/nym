import React from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';

const rootDomElem = document.getElementById('root');

if (rootDomElem) {
  const root = createRoot(rootDomElem);
  root.render(<App />);
}
