import React from 'react'
import { Card, Theme, Typography } from '@material-ui/core'
import { ErrorOutline } from '@material-ui/icons'
import { Alert } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'

export const SendError = ({ message }: { message?: string }) => {
  const theme: Theme = useTheme()

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        width: '100%',
      }}
    >
      <>
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: theme.spacing(4),
          }}
        >
          <ErrorOutline
            style={{ fontSize: 50, color: theme.palette.error.main }}
          />
          <Typography>Transaction failed</Typography>
        </div>

        <Card
          variant="outlined"
          style={{ width: '100%', padding: theme.spacing(2) }}
        >
          <Alert severity="error" data-testid="transaction-error">
            An error occured during the request {message}
          </Alert>
        </Card>
      </>
    </div>
  )
}
