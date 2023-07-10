import * as React from 'react';
import { AppBar as MUIAppBar } from '@mui/material/';
import Box from '@mui/material/Box';
import Toolbar from '@mui/material/Toolbar';

export const AppBar = ({ Action }: { Action: React.ReactNode }) => (
  <Box sx={{ flexGrow: 1 }}>
    <MUIAppBar position="static" elevation={0} sx={{ bgcolor: 'rgba(103, 80, 164, 0.14)' }}>
      <Toolbar variant="dense">{Action}</Toolbar>
    </MUIAppBar>
  </Box>
);

export default AppBar;
