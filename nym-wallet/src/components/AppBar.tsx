import React, { useContext } from 'react';
import { AppBar as MuiAppBar, Grid, IconButton, Toolbar } from '@mui/material';
import { useHistory } from 'react-router-dom';
import { Logout } from '@mui/icons-material';
import TerminalIcon from '@mui/icons-material/Terminal';
import { AppContext } from '../context/main';
import { NetworkSelector } from './NetworkSelector';
import { Node as NodeIcon } from '../svg-icons/node';
import { MultiAccounts } from './Accounts';
import { config } from '../../config';

export const AppBar = () => {
  const { showSettings, logOut, handleShowSettings, handleShowTerminal, appEnv } = useContext(AppContext);
  const history = useHistory();
  return (
    <MuiAppBar position="sticky" sx={{ boxShadow: 'none', bgcolor: 'transparent' }}>
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
            {(appEnv?.SHOW_TERMINAL || config.IS_DEV_MODE) && (
              <Grid item>
                <IconButton size="small" onClick={handleShowTerminal} sx={{ color: 'nym.background.dark' }}>
                  <TerminalIcon fontSize="small" />
                </IconButton>
              </Grid>
            )}
            <Grid item>
              <IconButton
                onClick={handleShowSettings}
                sx={{ color: showSettings ? 'primary.main' : 'nym.background.dark' }}
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
                  history.push('/');
                }}
                sx={{ color: 'nym.background.dark' }}
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
