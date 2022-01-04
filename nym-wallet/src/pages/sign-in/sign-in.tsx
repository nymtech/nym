import React, { useContext, useState } from 'react'
import { Button, CircularProgress, Grid, Stack, Link, TextField, Typography, Alert, SvgIcon } from '@mui/material'
import { styled } from '@mui/material/styles'
import Logo from '../../images/logo-background.svg'
import { signInWithMnemonic } from '../../requests'
import { ClientContext } from '../../context/main'

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
      const res = await signInWithMnemonic(mnemonic || '')
      setIsLoading(false)
      logIn(res)
    } catch (e: any) {
      setIsLoading(false)
      setInputError(e)
    }
  }

  return (
    <Stack spacing={3} alignItems="center" sx={{ width: '80%' }}>
      <Logo width={80} />
      <Typography sx={{ color: 'common.white' }}>Enter Mnemonic and sign in</Typography>

      <Grid container direction="column" spacing={3} component="form">
        <Grid item style={{ paddingTop: 0 }}>
          <StyledInput
            value={mnemonic}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setMnemonic(e.target.value)}
            size="medium"
            variant="outlined"
            margin="normal"
            required
            fullWidth
            id="mnemonic"
            label="BIP-39 Mnemonic"
            name="mnemonic"
            autoComplete="mnemonic"
            autoFocus
            disabled={isLoading}
            sx={{ m: 0 }}
          />
        </Grid>
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
            {!isLoading ? 'Sign In' : 'Signing in'}
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

      <div>
        <Typography sx={{ color: 'common.white' }} component="span">
          Don't have an account?
        </Typography>{' '}
        <Link href="#" onClick={showCreateAccount}>
          Create one now
        </Link>
      </div>
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
