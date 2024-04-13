import * as React from 'react'
import { FallbackProps } from 'react-error-boundary'
import { Alert, AlertTitle, Container } from '@mui/material'
import { NymThemeProvider } from '@nymproject/mui-theme'
import { NymLogo } from '@nymproject/react/logo/NymLogo'

export const ErrorBoundaryContent: FCWithChildren<FallbackProps> = ({
  error,
}) => (
  <NymThemeProvider mode="dark">
    <Container sx={{ py: 4 }}>
      <NymLogo height="75px" width="75px" />
      <h1>Oh no! Sorry, something went wrong</h1>
      <Alert severity="error" data-testid="error-message">
        <AlertTitle>{error.name}</AlertTitle>
        {error.message}
      </Alert>
      {process.env.NODE_ENV === 'development' && (
        <Alert severity="info" sx={{ mt: 2 }} data-testid="stack-trace">
          <AlertTitle>Stack trace</AlertTitle>
          {error.stack}
        </Alert>
      )}
    </Container>
  </NymThemeProvider>
)
