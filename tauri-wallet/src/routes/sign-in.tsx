import React, { useContext, useState } from 'react'
import {
  TextField,
  CircularProgress,
  Button,
  Typography,
  Grid,
  Link,
  Theme,
  Card,
  Divider,
} from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'
import { ArrowBack, CheckCircleOutline } from '@material-ui/icons'
import logo from '../images/logo-background.svg'
import logo_alt from '../images/logo.png'
import { ClientContext } from '../context/main'
import { theme } from '../theme'
import { createAccount, signInWithMnemonic } from '../requests'
import { TCreateAccount } from '../types'
import { CopyToClipboard } from '../components'

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
  const [mnemonic, setMnemonic] = useState<string>('')
  const [inputError, setInputError] = useState<string>()
  const [isLoading, setIsLoading] = useState(false)

  const { logIn } = useContext(ClientContext)

  const theme: Theme = useTheme()

  const handleSignIn = async (e: React.FormEvent<HTMLFormElement>) => {
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
    <SignInCard>
      <>
        <Typography variant="h4" data-testid="sign-in">Sign in</Typography>
        <form noValidate onSubmit={handleSignIn}>
          <Grid container direction="column" spacing={1}>
            <Grid item>
              <TextField
                style={{ background: 'white' }}
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
      </>
    </SignInCard>
  )
}

const SignInCard: React.FC = ({ children }) => {
  const theme: Theme = useTheme()
  return (
    <>
      <Card
        style={{
          width: 600,
          padding: theme.spacing(6, 10),
          borderRadius: theme.shape.borderRadius,
          position: 'relative',
          minHeight: 350,
        }}
      >
        <img
          src={logo_alt}
          style={{
            position: 'absolute',
            width: 425,
            filter: 'grayscale(100%)',
            opacity: 0.1,
            top: '50%',
            left: '50%',
            transform: 'translate(0%, -50%)',
          }}
        />
        {children}
      </Card>
    </>
  )
}

const CreateAccountContent = ({ showSignIn }: { showSignIn: () => void }) => {
  const [accountDetails, setAccountDetails] = useState<TCreateAccount>()
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<Error>()

  const handleCreateAccount = async () => {
    setIsLoading(true)
    setError(undefined)
    try {
      const res = await createAccount()
      setTimeout(() => {
        setAccountDetails(res)
        setIsLoading(false)
      }, 2500)
    } catch (e: any) {
      setError(e)
    }
  }
  return (
    <SignInCard>
      <Typography variant="h4">Create wallet</Typography>
      <Typography color="textSecondary">
        Create a new wallet to start using the Nym network
      </Typography>
      <Grid
        container
        direction="column"
        spacing={3}
        style={{ marginTop: theme.spacing(3) }}
      >
        <Grid item container justifyContent="center">
          {isLoading && <CircularProgress size={48} />}
          {!isLoading && accountDetails && (
            <>
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  justifyContent: 'center',
                  marginBottom: theme.spacing(4),
                }}
              >
                <CheckCircleOutline
                  style={{
                    fontSize: 50,
                    color: theme.palette.success.main,
                    marginBottom: theme.spacing(1),
                  }}
                />
                <Typography>Wallet setup complete</Typography>
              </div>
              <Alert severity="info" style={{ marginBottom: theme.spacing(2) }} data-testid="mnemonic-warning">
                Please store your <strong>mnemonic</strong> in a safe place.
                You'll need it to access your wallet
              </Alert>
              <Card
                variant="outlined"
                style={{
                  width: '100%',
                  padding: theme.spacing(2),
                }}
              >
                <Grid container direction="column" spacing={1}>
                  <Grid item>
                    <Typography style={{ color: theme.palette.grey[600] }}>
                      Mnemonic
                    </Typography>
                  </Grid>
                  <Grid item>
                    <Typography data-testid="mnemonic-phrase">{accountDetails.mnemonic}</Typography>
                    <div
                      style={{ display: 'flex', justifyContent: 'flex-end' }}
                    >
                      <CopyToClipboard text={accountDetails.mnemonic} />
                    </div>
                  </Grid>
                  <Grid item>
                    <Divider light />
                  </Grid>
                  <Grid item>
                    <Typography style={{ color: theme.palette.grey[600] }}>
                      Address
                    </Typography>
                  </Grid>
                  <Grid item>
                    <Typography data-testid="wallet-address">{accountDetails.client_address}</Typography>
                  </Grid>
                </Grid>
              </Card>
            </>
          )}
        </Grid>
        {error && (
          <Grid item style={{ marginTop: theme.spacing(1) }}>
            <Alert severity="error" data-testid="error">{error}</Alert>
          </Grid>
        )}
        <Grid item>
          {!accountDetails && (
            <Button
              onClick={handleCreateAccount}
              fullWidth
              variant="contained"
              color="primary"
              type="submit"
              data-testid="create-button"
              disableElevation
              style={{ marginBottom: theme.spacing(1) }}
              disabled={isLoading}
            >
              Create
            </Button>
          )}
          <Button
            fullWidth
            variant="text"
            onClick={showSignIn}
            data-testid="sign-in-button"
            startIcon={<ArrowBack />}
          >
            Sign in
          </Button>
        </Grid>
      </Grid>
    </SignInCard>
  )
}
