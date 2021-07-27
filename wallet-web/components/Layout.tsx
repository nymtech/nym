import React from 'react'
import { Grid, Theme, useTheme } from '@material-ui/core'

export const Layout = ({ children }: { children: React.ReactElement }) => {
  const theme: Theme = useTheme()

  return (
    <div
      style={{
        padding: theme.spacing(5),
      }}
    >
      <Grid container justify="center">
        <Grid item xs={12} md={6} lg={4}>
          {children}
        </Grid>
      </Grid>
    </div>
  )
}
