import React from 'react'
import Typography from '@material-ui/core/Typography'
import Grid from '@material-ui/core/Grid'
import { CircularProgress } from '@material-ui/core'
import { Alert, AlertTitle } from '@material-ui/lab'

type ConfirmationProps = {
  isLoading: boolean
  progressMessage: string
  successMessage: string
  failureMessage: string
  error: Error
}

export default function Confirmation({
  isLoading,
  progressMessage,
  successMessage,
  failureMessage,
  error,
}: ConfirmationProps) {
  return isLoading ? (
    <>
      <Typography variant='h6' gutterBottom>
        {progressMessage}
      </Typography>
      <Grid item xs={12} sm={6}>
        <CircularProgress />
      </Grid>
    </>
  ) : (
    <>
      {error === null ? (
        <Alert severity='success'>{successMessage}</Alert>
      ) : (
        <Alert severity='error'>
          <AlertTitle>{error.name}</AlertTitle>
          <strong>{failureMessage}</strong> - {error.message}
        </Alert>
      )}
    </>
  )
}
