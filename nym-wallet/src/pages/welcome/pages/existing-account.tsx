import React, { useContext, useState } from 'react'
import { Alert, Button, Stack, TextField } from '@mui/material'
import { Subtitle } from '../components'
import { ClientContext } from '../../../context/main'
import { signInWithMnemonic } from '../../../requests'

export const ExistingAccount: React.FC<{ page: 'existing account' }> = () => {
  const [mnemonic, setMnemonic] = useState<string>()
  const [inputError, setInputError] = useState<string>()
  const [isLoading, setIsLoading] = useState(false)

  const { logIn } = useContext(ClientContext)

  const handleSignIn = async (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault()

    setIsLoading(true)
    setInputError(undefined)

    try {
      const res = await signInWithMnemonic(mnemonic || '')
      setIsLoading(false)
      logIn(res)
    } catch (e: any) {
      setIsLoading(false)
      setInputError(e)
    }
  }

  return (
    <Stack spacing={3} sx={{ width: 400 }} alignItems="center">
      <Subtitle subtitle="Enter your mnemonic from existing wallet" />
      <TextField value={mnemonic} onChange={(e) => setMnemonic(e.target.value)} multiline rows={5} fullWidth />
      {inputError && (
        <Alert severity="error" variant="outlined" data-testid="error" sx={{ color: 'error.light', width: '100%' }}>
          {inputError}
        </Alert>
      )}
      <Button variant="contained" size="large" fullWidth onClick={handleSignIn}>
        Next
      </Button>
    </Stack>
  )
}
