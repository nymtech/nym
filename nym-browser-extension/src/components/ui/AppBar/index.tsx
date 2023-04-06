import * as React from 'react';
import { AppBar as MUIAppBar } from '@mui/material/';
import Box from '@mui/material/Box';
import Toolbar from '@mui/material/Toolbar';
import IconButton from '@mui/material/IconButton';
import MenuIcon from '@mui/icons-material/Menu';

export default function AppBar({ onMenuOpen }: { onMenuOpen: () => void }) {
  return (
    <Box sx={{ flexGrow: 1 }}>
      <MUIAppBar position="static" elevation={0} sx={{ bgcolor: 'rgba(103, 80, 164, 0.14)' }}>
        <Toolbar variant="dense">
          <IconButton onClick={onMenuOpen}>
            <MenuIcon />
          </IconButton>
        </Toolbar>
      </MUIAppBar>
    </Box>
  );
}
