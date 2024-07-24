import { useContext } from 'react';
import { AppBar as MuiAppBar, Grid, IconButton, Toolbar } from '@mui/material';
import { useNavigate } from 'react-router-dom';
import { Logout, SettingsOutlined as SettingsIcon } from '@mui/icons-material';
import { AppContext } from '../context/main';
import { NetworkSelector } from './NetworkSelector';
import { MultiAccounts } from './Accounts';

export const AppBar = () => {
  const { logOut } = useContext(AppContext);
  const navigate = useNavigate();

  return (
    <MuiAppBar position="sticky" sx={{ boxShadow: 'none', bgcolor: 'transparent', backgroundImage: 'none', mt: 3 }}>
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
              <IconButton size="small" onClick={() => navigate('/settings')} sx={{ color: 'text.primary' }}>
                <SettingsIcon fontSize="small" />
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
