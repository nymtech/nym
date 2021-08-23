import { Card, CircularProgress, Theme, Typography } from '@material-ui/core'
import { ErrorOutline } from '@material-ui/icons'
import { Alert } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'
import React, { useEffect, useState } from 'react'

export const SendError = ({ onFinish }: { onFinish: () => void }) => {
  const theme: Theme = useTheme()
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    setTimeout(() => {
      setIsLoading(false)
      onFinish()
    }, 3000)
  }, [])

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
      {isLoading ? (
        <CircularProgress size={48} />
      ) : (
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
            <Alert severity="error">An error occured during the request</Alert>
          </Card>
        </>
      )}
    </div>
  )
}
