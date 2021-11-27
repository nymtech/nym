import React, { useContext } from 'react'
import {
  AppBar as MuiAppBar,
  Divider,
  Grid,
  IconButton,
  List,
  ListItem,
  ListItemText,
  Toolbar,
  Typography,
} from '@mui/material'
import { ExitToApp } from '@mui/icons-material'
import { ClientContext } from '../context/main'
import { Box } from '@mui/system'
import { CopyToClipboard } from '.'

export const AppBar = () => {
  const { userBalance, logOut, clientDetails } = useContext(ClientContext)

  return (
    <MuiAppBar
      position="sticky"
      sx={{ boxShadow: 'none', bgcolor: 'nym.background.light' }}
    >
      <Toolbar>
        <Grid
          container
          justifyContent="space-between"
          alignItems="center"
          flexWrap="nowrap"
        >
          <Grid container item alignItems="center">
            <Grid item>
              <AppBarItem
                primaryText="Balance"
                secondaryText={userBalance.balance?.printable_balance}
              />
            </Grid>
            <Divider orientation="vertical" variant="middle" flexItem />
            <Grid item>
              <AppBarItem
                primaryText="Address"
                secondaryText={clientDetails?.client_address}
                Action={
                  <CopyToClipboard text={clientDetails?.client_address} />
                }
              />
            </Grid>
          </Grid>
          <Grid item>
            <IconButton onClick={logOut} sx={{ color: 'nym.background.dark' }}>
              <ExitToApp />
            </IconButton>
          </Grid>
        </Grid>
      </Toolbar>
    </MuiAppBar>
  )
}

const AppBarItem: React.FC<{
  primaryText: string
  secondaryText?: string
  Action?: React.ReactNode
}> = ({ primaryText, secondaryText = '', Action }) => {
  return (
    <Box sx={{ p: 1 }}>
      <Typography variant="body2" component="span" sx={{ color: 'grey.600' }}>
        {primaryText}:
      </Typography>{' '}
      <Typography variant="body2" component="span" color="nym.background.dark">
        {secondaryText}
      </Typography>
      {Action}
    </Box>
  )
}
