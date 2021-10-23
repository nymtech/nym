import * as React from 'react';
import { NewNav } from './components/NewNav';
import { Routes } from './routes/index';

export const App: React.FC = () => (
  <NewNav>
    <Routes />
  </NewNav>
);
