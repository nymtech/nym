import React from 'react'
import {
  Alert,
  AlertTitle,
  CircularProgress,
  Grid,
  Typography,
} from '@mui/material'

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
