import React, { useEffect, useState } from 'react'
import { Alert, Button, Card, CardActions, CardContent, CardHeader, Stack, Typography } from '@mui/material'
import { createAccount } from '../../requests'
import { TCreateAccount } from '../../types'
import { CopyToClipboard } from '../../components'

export const CreateAccountContent: React.FC<{ page: 'legacy create account'; showSignIn: () => void }> = ({
  showSignIn,
}) => {
  const [accountDetails, setAccountDetails] = useState<TCreateAccount>()
  const [error, setError] = useState<Error>()

  const handleCreateAccount = async () => {
    setError(undefined)
    try {
      const account = await createAccount()
      setAccountDetails(account)
    } catch (e: any) {
      setError(e)
    }
  }

  useEffect(() => {
    handleCreateAccount()
  }, [])

  return (
    <Stack spacing={4} alignItems="center" sx={{ width: 700 }}>
      <Typography sx={{ color: 'common.white' }} variant="h4">
        Congratulations
      </Typography>
      <Typography sx={{ color: 'common.white' }} variant="h6">
        Account setup complete!
      </Typography>
      <Alert severity="info" variant="outlined" sx={{ color: 'info.light' }} data-testid="mnemonic-warning">
        <Typography>Please store your mnemonic in a safe place. You'll need it to access your account!</Typography>
      </Alert>
      <Card variant="outlined" sx={{ bgcolor: 'transparent', p: 2, borderColor: 'common.white' }}>
        <CardHeader sx={{ color: 'common.white' }} title="Mnemonic" />
        <CardContent sx={{ color: 'common.white' }} data-testid="mnemonic-phrase">
          {accountDetails?.mnemonic}
        </CardContent>
        <CardActions sx={{ justifyContent: 'flex-end' }}>
          <CopyToClipboard text={accountDetails?.mnemonic || ''} light />
        </CardActions>
      </Card>
      {error && (
        <Alert severity="error" variant="outlined">
          {error}
        </Alert>
      )}
      <Button variant="contained" onClick={showSignIn} data-testid="sign-in-button" size="large" sx={{ width: 360 }}>
        Sign in
      </Button>
    </Stack>
  )
}
