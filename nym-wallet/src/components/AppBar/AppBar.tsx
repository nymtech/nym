import React, { useContext } from 'react';
import TerminalIcon from '@mui/icons-material/Terminal';
import { AppBar as MuiAppBar, Grid, IconButton, Toolbar } from '@mui/material';
import { Send } from 'src/components/Send';
import { config } from '../../config';
import { AppContext } from '../../context/main';
import { MultiAccounts } from '../Accounts';
import { NetworkSelector } from '../NetworkSelector';
import { AppMenu } from './AppMenu';

export const AppBar = () => {
  const { handleShowTerminal, appEnv } = useContext(AppContext);

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
          <Grid item container justifyContent="flex-end" alignItems="center" md={12} lg={5} spacing={2}>
            {(appEnv?.SHOW_TERMINAL || config.IS_DEV_MODE) && (
              <Grid item>
                <IconButton size="small" onClick={handleShowTerminal} sx={{ color: 'nym.background.dark' }}>
                  <TerminalIcon fontSize="small" />
                </IconButton>
              </Grid>
            )}
            <Grid item>
              <Send />
            </Grid>
            <Grid item>
              <AppMenu />
            </Grid>
            {/* <Grid item>
              <IconButton
                onClick={handleShowSettings}
                sx={{ color: showSettings ? 'primary.main' : 'nym.background.dark' }}
                size="small"
              >
                <NodeIcon fontSize="small" />
              </IconButton>
            </Grid>
            <Grid item>
              <IconButton size="small" onClick={logOut} sx={{ color: 'nym.background.dark' }}>
                <Logout fontSize="small" />
              </IconButton>
            </Grid> */}
          </Grid>
        </Grid>
      </Toolbar>
    </MuiAppBar>
  );
};
