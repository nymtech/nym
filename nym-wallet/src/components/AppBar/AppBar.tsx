import React, { useContext } from 'react';
import { Logout } from '@mui/icons-material';
import TerminalIcon from '@mui/icons-material/Terminal';
import ModeNightOutlinedIcon from '@mui/icons-material/ModeNightOutlined';
import LightModeOutlinedIcon from '@mui/icons-material/LightModeOutlined';
import { AppBar as MuiAppBar, Grid, IconButton, Toolbar, FormGroup, FormControlLabel, Switch } from '@mui/material';
import { Node } from 'src/svg-icons/node';
import { config } from '../../config';
import { AppContext } from '../../context/main';
import { MultiAccounts } from '../Accounts';
import { NetworkSelector } from '../NetworkSelector';

export const AppBar = () => {
  const { showSettings, handleShowTerminal, appEnv, handleShowSettings, logOut, mode, handleSwitchMode } =
    useContext(AppContext);

  return (
    <MuiAppBar position="sticky" sx={{ boxShadow: 'none', bgcolor: 'transparent', backgroundImage: 'none' }}>
      <Toolbar disableGutters>
        <Grid container justifyContent="space-between" alignItems="center" flexWrap="nowrap">
          <Grid item container alignItems="center" spacing={1}>
            <Grid item>
              <MultiAccounts />
            </Grid>
            <Grid item>
              <NetworkSelector />
            </Grid>
          </Grid>
          <Grid item container justifyContent="flex-end" md={12} lg={5} spacing={2}>
            <Grid item>
              <IconButton size="small" onClick={handleSwitchMode} sx={{ color: 'text.primary' }}>
                {mode === 'light' ? (
                  <ModeNightOutlinedIcon fontSize="small" />
                ) : (
                  <LightModeOutlinedIcon fontSize="small" />
                )}
              </IconButton>
            </Grid>
            {(appEnv?.SHOW_TERMINAL || config.IS_DEV_MODE) && (
              <Grid item>
                <IconButton size="small" onClick={handleShowTerminal} sx={{ color: 'text.primary' }}>
                  <TerminalIcon fontSize="small" />
                </IconButton>
              </Grid>
            )}
            <Grid item>
              <IconButton
                onClick={handleShowSettings}
                sx={{ color: showSettings ? 'primary.main' : 'text.primary' }}
                size="small"
              >
                <Node fontSize="small" />
              </IconButton>
            </Grid>
            <Grid item>
              <IconButton size="small" onClick={logOut} sx={{ color: 'text.primary' }}>
                <Logout fontSize="small" />
              </IconButton>
            </Grid>
          </Grid>
        </Grid>
      </Toolbar>
    </MuiAppBar>
  );
};
