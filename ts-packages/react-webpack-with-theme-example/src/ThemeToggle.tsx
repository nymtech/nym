import * as React from 'react';
import { Button, Typography } from '@mui/material';
import DarkModeIcon from '@mui/icons-material/DarkMode';
import LightModeIcon from '@mui/icons-material/LightMode';
import { useAppContext } from './context';

export const ThemeToggle: React.FC = () => {
  const { mode, toggleMode } = useAppContext();
  return (
    <Button variant="outlined" color="secondary" onClick={toggleMode} sx={{ display: 'flex', alignItems: 'centre' }}>
      {mode === 'dark' ? (
        <DarkModeIcon sx={{ color: (theme) => theme.palette.text.secondary }} />
      ) : (
        <LightModeIcon sx={{ color: (theme) => theme.palette.text.secondary }} />
      )}
      <Typography ml={1} color={(theme) => theme.palette.primary.light}>
        Switch to {mode === 'dark' ? 'light mode' : 'dark mode'}
      </Typography>
    </Button>
  );
};
