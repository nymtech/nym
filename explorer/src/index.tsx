import * as React from 'react';
import ReactDOM from 'react-dom';
import { BrowserRouter as Router } from 'react-router-dom';
import { App } from './App';
import { MainContextProvider } from './context/main';
import './styles.css';
import { NetworkExplorerThemeProvider } from './theme';

ReactDOM.render(
  <MainContextProvider>
    <NetworkExplorerThemeProvider>
      <Router>
        <App />
      </Router>
    </NetworkExplorerThemeProvider>
  </MainContextProvider>,
  document.getElementById('app'),
);
