import React, { useCallback, useState } from 'react';
import { Box, IconButton } from '@mui/material';
import MenuIcon from '@mui/icons-material/Menu';
import { AppBar, BackButton, MenuDrawer } from 'src/components/ui';
import { useLocation } from 'react-router-dom';

const layoutStyle = {
  display: 'grid',
  gridTemplateColumns: '1fr',
  gridTemplateRows: '50px 1fr',
  gridColumnGap: '0px',
  gridRowGap: '0px',
};

export const PageLayout = ({ children }: { children: React.ReactNode }) => {
  const [menuOpen, setMenuOpen] = useState(false);

  const location = useLocation();

  const MenuAction = useCallback(
    () => (
      <IconButton onClick={() => setMenuOpen(true)}>
        <MenuIcon />
      </IconButton>
    ),
    [],
  );

  const Action = location.pathname.includes('balance') ? MenuAction : BackButton;

  return (
    <Box sx={layoutStyle}>
      <AppBar Action={<Action />} />
      <MenuDrawer open={menuOpen} onClose={() => setMenuOpen(false)} />
      <Box sx={{ p: 2 }}>{children}</Box>
    </Box>
  );
};
