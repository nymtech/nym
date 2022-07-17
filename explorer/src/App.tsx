import * as React from 'react';
import { Nav } from './components/Nav';
import { MobileNav } from './components/MobileNav';
import { Routes } from './routes/index';
import { useIsMobile } from './hooks/useIsMobile';

export const App: React.FC = () => {
  const isMobile = useIsMobile();

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
