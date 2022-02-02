import React, { useContext, useState } from 'react'
import { Button, CircularProgress, Grid, Stack, TextField, Typography, Alert } from '@mui/material'
import { styled } from '@mui/material/styles'
import { signInWithMnemonic } from '../../requests'
import { ClientContext } from '../../context/main'
import { NymLogo } from '../../components'

export const SignInContent: React.FC<{ showCreateAccount: () => void }> = ({ showCreateAccount }) => {
  const [mnemonic, setMnemonic] = useState<string>('')
  const [inputError, setInputError] = useState<string>()
  const [isLoading, setIsLoading] = useState(false)

  const { logIn } = useContext(ClientContext)

  const handleSignIn = async (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault()

    setIsLoading(true)
    setInputError(undefined)

    try {
      await signInWithMnemonic(mnemonic || '')
      setIsLoading(false)
      logIn()
    } catch (e: any) {
      setIsLoading(false)
      setInputError(e)
    }
  }

  return (
    <Stack spacing={3} alignItems="center" sx={{ width: '80%' }}>
      <NymLogo />
      <Typography sx={{ color: 'common.white', fontWeight: 600 }}>Welcome to NYM</Typography>
      <Typography variant="caption" sx={{ color: 'grey.800', textTransform: 'uppercase', letterSpacing: 4 }}>
        Next generation of privacy
      </Typography>
      <Grid container direction="column" spacing={2}>
        <Grid item>
          <Button
            fullWidth
            variant="contained"
            color="primary"
            disabled={isLoading}
            endIcon={isLoading && <CircularProgress size={20} />}
            disableElevation
            size="large"
            onClick={handleSignIn}
            type="submit"
          >
            Create Account
          </Button>
        </Grid>
        <Grid item>
          <Button fullWidth variant="outlined" size="large">
            Use Existing Account
          </Button>
        </Grid>
        {inputError && (
          <Grid item sx={{ mt: 1 }}>
            <Alert severity="error" variant="outlined" data-testid="error" sx={{ color: 'error.light' }}>
              {inputError}
            </Alert>
          </Grid>
        )}
      </Grid>
    </Stack>
  )
}

const StyledInput = styled((props) => <TextField {...props} />)(({ theme }) => ({
  '& input': {
    color: theme.palette.nym.text.light,
  },
  '& label': {
    color: theme.palette.nym.text.light,
  },
  '& label.Mui-focused': {
    color: theme.palette.primary.main,
  },
  '& .MuiOutlinedInput-root': {
    '& fieldset': {
      borderColor: theme.palette.common.white,
    },
    '&:hover fieldset': {
      borderColor: theme.palette.primary.main,
    },
  },
}))
