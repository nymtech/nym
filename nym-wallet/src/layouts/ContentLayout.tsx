import React from 'react'
import { Box, Grid } from '@mui/material'

export const Layout = ({ children }: { children: React.ReactElement }) => {
  return (
    <Box
      sx={{
        padding: 5,
        width: '100%',
        height: '100%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <Grid container justifyContent="center" style={{ margin: 'auto' }}>
        <Grid item xs={12} md={8} xl={6}>
          {children}
        </Grid>
      </Grid>
    </Box>
  )
}
