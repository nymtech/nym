import React, { useState } from 'react';
import { Box } from '@mui/material';
import { AppBar, MenuDrawer } from 'src/components/ui';

const layoutStyle = {
  display: 'grid',
  gridTemplateColumns: '1fr',
  gridTemplateRows: '50px 1fr',
  gridColumnGap: '0px',
  gridRowGap: '0px',
};

export const PageLayout = ({ children }: { children: React.ReactNode }) => {
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <Box sx={layoutStyle}>
      <AppBar onMenuOpen={() => setMenuOpen(true)} />
      <MenuDrawer open={menuOpen} onClose={() => setMenuOpen(false)} />
      <Box sx={{ p: 2 }}>{children}</Box>
    </Box>
  );
};
