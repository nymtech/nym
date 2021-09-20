import * as React from 'react';
import { useLocation } from 'react-router-dom';
import { Nav } from './components/Nav';
import { Routes } from './routes/index';

export const App: React.FC = () => (
  <Nav>
    <Routes />
  </Nav>
);
