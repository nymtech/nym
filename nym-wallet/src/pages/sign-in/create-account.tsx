import React, { useEffect, useState } from 'react'
import {
  Alert,
  Button,
  Card,
  CardActions,
  CardContent,
  CardHeader,
  CircularProgress,
  Stack,
  Typography,
} from '@mui/material'
import { Box } from '@mui/system'
import { ArrowBack } from '@mui/icons-material'
import { createAccount } from '../../requests'
import { TCreateAccount } from '../../types'
import logo from '../../images/logo-background.svg'
import { CopyToClipboard } from '../../components'

export const CreateAccountContent: React.FC<{ showSignIn: () => void }> = ({ showSignIn }) => {
  const [accountDetails, setAccountDetails] = useState<TCreateAccount>()
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<Error>()

  const handleCreateAccount = async () => {
    setIsLoading(true)
    setError(undefined)
    try {
      const res = await createAccount()
      console.log(res)
      setTimeout(() => {
        setAccountDetails(res)
        setIsLoading(false)
      }, 2500)
    } catch (e: any) {
      setError(e)
    }
  }

  useEffect(() => {
    handleCreateAccount()
  }, [])

  if (isLoading) return <CircularProgress size={70} />

  return (
    <Stack spacing={4} alignItems="center">
      <img src={logo} width={80} />
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
      <Box sx={{ textAlign: 'center' }}>
        <Typography sx={{ color: 'common.white' }}>Address:</Typography>
        <Typography sx={{ color: 'common.white' }} data-testid="wallet-address">
          {accountDetails?.account.client_address}
        </Typography>
      </Box>
      {error && (
        <Alert severity="error" variant="outlined">
          {error}
        </Alert>
      )}
      <Button
        variant="contained"
        onClick={showSignIn}
        data-testid="sign-in-button"
        startIcon={<ArrowBack />}
        size="large"
        sx={{ width: 360 }}
      >
        Back to Sign in
      </Button>
    </Stack>
  )
}
