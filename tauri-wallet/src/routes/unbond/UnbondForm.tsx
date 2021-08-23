import React, { useState } from 'react'
import { Alert } from '@material-ui/lab'
import { Button, Theme } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'

export const UnbondForm = () => {
  const theme: Theme = useTheme()
  return (
    <div>
      <Alert severity="info" style={{ margin: theme.spacing(3) }}>
        You don't currently have a bonded node
      </Alert>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          borderTop: `1px solid ${theme.palette.grey[200]}`,
          background: theme.palette.grey[100],
          padding: theme.spacing(2),
        }}
      >
        <Button
          variant="contained"
          color="primary"
          type="submit"
          disableElevation
        >
          Unbond
        </Button>
      </div>
    </div>
  )
}
