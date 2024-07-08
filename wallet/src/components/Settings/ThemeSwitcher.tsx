import React, { useContext } from 'react';
import { Stack, Switch, Typography } from '@mui/material';
import ModeNightOutlinedIcon from '@mui/icons-material/ModeNightOutlined';
import LightModeOutlinedIcon from '@mui/icons-material/LightModeOutlined';
import { AppContext } from '../../context';

const ThemeSwitcher = () => {
  const { mode, handleSwitchMode } = useContext(AppContext);

  return (
    <Stack direction="row" alignItems="center">
      <LightModeOutlinedIcon fontSize="small" sx={{ mr: 1 }} />
      <Typography>Light mode</Typography>
      <Switch checked={mode === 'dark'} onChange={handleSwitchMode} inputProps={{ 'aria-label': 'controlled' }} />
      <ModeNightOutlinedIcon fontSize="small" sx={{ transform: 'rotate(145deg)', mr: 1 }} />
      <Typography>Dark mode</Typography>
    </Stack>
  );
};

export default ThemeSwitcher;
