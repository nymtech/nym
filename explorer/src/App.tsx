import * as React from 'react';
import { Nav } from './components/Nav';
import { Routes } from './routes/index';

export const App: React.FC = () => (
  <Nav>
    <Routes />
  </Nav>
);
