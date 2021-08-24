import React, { useState } from 'react'
import {
  TextField,
  LinearProgress,
  Button,
  Typography,
  Grid,
  Link,
  Theme,
  Card,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import logo from '../images/logo.png'
import { useHistory } from 'react-router-dom'
import { invoke } from '@tauri-apps/api'

export const SignIn = () => {
  const handleSignIn = (e: React.FormEvent<any>) => {
    e.preventDefault()

    history.push('/bond')
  }

  const [loading, setLoading] = useState(false)
  const theme: Theme = useTheme()
  const history = useHistory()
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
                />
              </Grid>
              <Grid item>
                <Button
                  fullWidth
                  variant="contained"
                  color="primary"
                  type="submit"
                  disabled={loading}
                  disableElevation
                >
                  Sign In
                </Button>
              </Grid>
              <Grid item style={{ marginTop: theme.spacing(1) }}>
                <Typography variant="body2" component="span">
                  Don't have an account?
                </Typography>{' '}
                <Link>Create one</Link>
              </Grid>
            </Grid>
          </form>
        </Card>
      </div>
    </div>
  )
}
