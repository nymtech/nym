import * as React from 'react';
import { useMediaQuery } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { Nav } from './components/Nav';
import { MobileNav } from './components/MobileNav';
import { Routes } from './routes/index';

export const App: React.FC = () => {
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));

  if (isMobile) {
    return (
      <MobileNav>
        <Routes />
      </MobileNav>
    );
  }
  return (
    <Nav>
      <Routes />
    </Nav>
  );
};
