import { Box } from '@mui/system';
import React, { useState } from 'react';
import AppBar from 'src/components/ui/AppBar';
import { MenuDrawer } from 'src/components/ui';

export const PageLayout = ({ children }: { children: React.ReactNode }) => {
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <>
      <AppBar onMenuOpen={() => setMenuOpen(true)} />
      <MenuDrawer open={menuOpen} onClose={() => setMenuOpen(false)} />
      <Box sx={{ p: 2 }}>{children}</Box>
    </>
  );
};
