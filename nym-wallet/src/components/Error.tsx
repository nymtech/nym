import React from 'react'
import { FallbackProps } from 'react-error-boundary'
import { Alert, AlertTitle } from '@material-ui/lab'
import { Button } from '@material-ui/core'

export const ErrorFallback = ({ error, resetErrorBoundary }: FallbackProps) => {
  return (
    <div>
      <Alert severity="error" data-testid="error-message">
        <AlertTitle>{error.name}</AlertTitle>
        {error.message}
      </Alert>
      <Alert severity="error" data-testid="stack-trace">
        <AlertTitle>Stack trace</AlertTitle>
        {error.stack}
      </Alert>
      <Button onClick={resetErrorBoundary}>Back to safety</Button>
    </div>
  )
}
