import React, { useContext } from 'react';
import { AppBar as MuiAppBar, Grid, IconButton, Toolbar } from '@mui/material';
import { useNavigate } from 'react-router-dom';
import { Logout } from '@mui/icons-material';
import TerminalIcon from '@mui/icons-material/Terminal';
import ModeNightOutlinedIcon from '@mui/icons-material/ModeNightOutlined';
import LightModeOutlinedIcon from '@mui/icons-material/LightModeOutlined';
import { AppContext } from '../context/main';
import { NetworkSelector } from './NetworkSelector';
import { Node as NodeIcon } from '../svg-icons/node';
import { MultiAccounts } from './Accounts';
import { config } from '../config';

export const AppBar = () => {
  const { logOut, handleShowTerminal, appEnv, handleShowSettings, showSettings, mode, handleSwitchMode } =
    useContext(AppContext);
  const navigate = useNavigate();

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
                {mode === 'dark' ? (
                  <LightModeOutlinedIcon fontSize="small" />
                ) : (
                  <ModeNightOutlinedIcon fontSize="small" sx={{ transform: 'rotate(180deg)' }} />
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
                <NodeIcon fontSize="small" />
              </IconButton>
            </Grid>
            <Grid item>
              <IconButton
                size="small"
                onClick={async () => {
                  await logOut();
                  navigate('/');
                }}
                sx={{ color: 'text.primary' }}
              >
                <Logout fontSize="small" />
              </IconButton>
            </Grid>
          </Grid>
        </Grid>
      </Toolbar>
    </MuiAppBar>
  );
};
