import React, { useContext, useState } from 'react'
import { useHistory } from 'react-router-dom'
import {
  TextField,
  CircularProgress,
  Button,
  Typography,
  Grid,
  Link,
  Theme,
  Card,
} from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'
import { ArrowBack } from '@material-ui/icons'
import { invoke } from '@tauri-apps/api'
import logo from '../images/logo-background.svg'
import { ClientContext } from '../context/main'
import { TClientDetails } from '../types/global'

export const SignIn = () => {
  const theme: Theme = useTheme()
  const [showCreateAccount, setShowCreateAccount] = useState(false)
  return (
    <div
      style={{
        height: '100vh',
        width: '100vw',
        display: 'grid',
        gridTemplateColumns: '400px auto',
        gridTemplateRows: '100%',
        gridColumnGap: '0px',
        gridRowGap: '0px',
      }}
    >
      <div
        style={{
          gridArea: '1 / 1 / 2 / 2',
          background: '#121726',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <img src={logo} style={{ width: 100 }} />
      </div>
      <div
        style={{
          gridArea: '1 / 2 / 2 / 3',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: theme.palette.grey[100],
        }}
      >
        {showCreateAccount ? (
          <CreateAccountContent
            showSignIn={() => setShowCreateAccount(false)}
          />
        ) : (
          <SignInContent showCreateAccount={() => setShowCreateAccount(true)} />
        )}
      </div>
    </div>
  )
}

const SignInContent = ({
  showCreateAccount,
}: {
  showCreateAccount: () => void
}) => {
  const [mnemonic, setMnemonic] = useState<string>(
    'alley mutual arrange escape army vacuum cherry ozone frame steel current smile dad subject primary foster lazy want perfect fury general eye cannon motor'
  )
  const [inputError, setInputError] = useState<string | undefined>()
  const [isLoading, setIsLoading] = useState(false)

  const { logIn } = useContext(ClientContext)

  const theme: Theme = useTheme()

  const handleSignIn = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()

    setIsLoading(true)
    setInputError(undefined)

    invoke('connect_with_mnemonic', { mnemonic })
      .then((res) => {
        setIsLoading(false)
        logIn(res as TClientDetails)
      })
      .catch((e) => {
        setIsLoading(false)
        setInputError(e)
      })
  }

  return (
    <Card
      style={{
        width: 600,
        padding: theme.spacing(6, 10),
        borderRadius: theme.shape.borderRadius,
      }}
    >
      <Typography variant="h4">Sign in</Typography>
      <form noValidate onSubmit={handleSignIn}>
        <Grid container direction="column" spacing={1}>
          <Grid item>
            <TextField
              value={mnemonic}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                setMnemonic(e.target.value)
              }
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
            />
          </Grid>
          <Grid item>
            <Button
              fullWidth
              variant="contained"
              color="primary"
              type="submit"
              disabled={isLoading}
              endIcon={isLoading && <CircularProgress size={20} />}
              disableElevation
            >
              {!isLoading ? 'Sign In' : 'Signing in'}
            </Button>
          </Grid>
          {inputError && (
            <Grid item style={{ marginTop: theme.spacing(1) }}>
              <Alert severity="error">{inputError}</Alert>
            </Grid>
          )}
          <Grid item style={{ marginTop: theme.spacing(1) }}>
            <Typography variant="body2" component="span">
              Don't have an account?
            </Typography>{' '}
            <Link href="#" onClick={showCreateAccount}>
              Create one
            </Link>
          </Grid>
        </Grid>
      </form>
    </Card>
  )
}

const CreateAccountContent = ({ showSignIn }: { showSignIn: () => void }) => {
  const theme: Theme = useTheme()
  return (
    <>
      <Card
        style={{
          width: 600,
          padding: theme.spacing(6, 10),
          borderRadius: theme.shape.borderRadius,
        }}
      >
        <img src={logo} />
        <Typography variant="h4">Create wallet</Typography>
        <Grid container direction="column" spacing={1}>
          <Grid item>
            <Typography color="textSecondary">
              Create an new wallet to start using the Nym network
            </Typography>
          </Grid>
          <Grid item>
            <Button
              fullWidth
              variant="contained"
              color="primary"
              type="submit"
              disableElevation
            >
              Create
            </Button>
          </Grid>
          <Grid item>
            <Button
              fullWidth
              variant="text"
              onClick={showSignIn}
              startIcon={<ArrowBack />}
            >
              Sign in
            </Button>
          </Grid>
        </Grid>
      </Card>
    </>
  )
}
