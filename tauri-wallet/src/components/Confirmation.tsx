import React from 'react'
import Typography from '@material-ui/core/Typography'
import Grid from '@material-ui/core/Grid'
import { CircularProgress } from '@material-ui/core'
import { Alert, AlertTitle } from '@material-ui/lab'

type ConfirmationProps = {
  isLoading: boolean
  progressMessage: string
  SuccessMessage: React.ReactNode
  failureMessage: string
  error: Error | null
}

export const Confirmation = ({
  isLoading,
  progressMessage,
  SuccessMessage,
  failureMessage,
  error,
}: ConfirmationProps) => {
  return isLoading ? (
    <>
      <Typography variant="h6" gutterBottom>
        {progressMessage}
      </Typography>
      <Grid item xs={12} sm={6}>
        <CircularProgress />
      </Grid>
    </>
  ) : (
    <>
      {error === null ? (
        SuccessMessage
      ) : (
        <Alert severity="error" data-testid="error-message">
          <AlertTitle>{error.name}</AlertTitle>
          <strong>{failureMessage}</strong> - {error.message}
        </Alert>
      )}
    </>
  )
}
