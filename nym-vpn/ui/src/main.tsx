import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { mockTauriIPC } from './dev/setup';
import './styles.css';

if (import.meta.env.MODE === 'dev-browser') {
  console.log('Running in dev-browser mode. Mocking tauri window and IPCs');
  mockTauriIPC();
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
