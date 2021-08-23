import React, { useState } from 'react'
import {
  TextField,
  LinearProgress,
  Button,
  Typography,
  Grid,
  Link,
  Theme,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import logo from '../images/logo.png'
import { useHistory } from 'react-router-dom'

export const SignIn = () => {
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
          borderTopRightRadius: 10,
          borderBottomRightRadius: 10,
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
        }}
      >
        <div style={{ width: 400 }}>
          <Typography variant="h4">Sign in</Typography>
          <form noValidate onSubmit={() => history.push('/balance')}>
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
                  style={{ background: 'white' }}
                />
              </Grid>
              <Grid item>
                <Button
                  fullWidth
                  variant="contained"
                  color="primary"
                  type="submit"
                  disabled={loading}
                  size="large"
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
        </div>
      </div>
    </div>
  )
}
