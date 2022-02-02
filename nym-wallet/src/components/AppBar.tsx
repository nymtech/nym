import React, { useContext } from 'react'
import { AppBar as MuiAppBar, Divider, Grid, IconButton, Toolbar, Typography, useMediaQuery } from '@mui/material'
import { Box } from '@mui/system'
import { Logout } from '@mui/icons-material'
import { ClientContext } from '../context/main'
import { CopyToClipboard, NetworkSelector } from '.'
import { Node as NodeIcon } from '../svg-icons/node'

export const AppBar = () => {
  const { userBalance, clientDetails, showSettings, logOut, handleShowSettings } = useContext(ClientContext)
  const matches = useMediaQuery('(min-width: 900px)')

  return (
    <MuiAppBar position="sticky" sx={{ boxShadow: 'none', bgcolor: 'nym.background.light' }}>
      <Toolbar>
        <Grid container justifyContent="space-between" alignItems="center" flexWrap="nowrap">
          <Grid container item alignItems="center">
            <Grid item>
              <AppBarItem primaryText="Balance" secondaryText={userBalance.balance?.printable_balance} />
            </Grid>
            {matches && (
              <>
                <Divider orientation="vertical" variant="middle" flexItem sx={{ mr: 1 }} />
                <Grid item>
                  <AppBarItem
                    primaryText="Address"
                    secondaryText={clientDetails?.client_address}
                    Action={<CopyToClipboard text={clientDetails?.client_address} iconButton />}
                  />
                </Grid>
              </>
            )}
          </Grid>
          <Grid item container justifyContent="flex-end" md={12} lg={5} spacing={2}>
            <Grid item>
              <NetworkSelector />
            </Grid>
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
              <IconButton size="small" onClick={logOut} sx={{ color: 'nym.background.dark' }}>
                <Logout fontSize="small" />
              </IconButton>
            </Grid>
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
    <Box sx={{ p: 1, mr: 1 }}>
      <Typography variant="body2" component="span" sx={{ color: 'grey.600' }}>
        {primaryText}:
      </Typography>{' '}
      <Typography variant="body2" component="span" color="nym.background.dark" sx={{ mr: 1 }}>
        {secondaryText}
      </Typography>
      {Action}
    </Box>
  )
}
