import React, { useState } from 'react'
import { Button, FormControl, Grid, IconButton, Stack, TextField } from '@mui/material'
import { VisibilityOff, Visibility } from '@mui/icons-material'
import { Subtitle, Title, PasswordStrength } from '../components'

export const CreatePassword = ({}: { page: 'create password' }) => {
  const [password, setPassword] = useState<string>('')
  const [confirmedPassword, setConfirmedPassword] = useState<string>()
  const [showPassword, setShowPassword] = useState(false)
  const [showConfirmedPassword, setShowConfirmedPassword] = useState(false)

  return (
    <>
      <Title title="Create password" />
      <Subtitle subtitle="Create a strong password. Min 8 characters, at least one capital letter, number and special symbol" />
      <Grid container justifyContent="center">
        <Grid item xs={6}>
          <FormControl fullWidth>
            <Stack spacing={2}>
              <TextField
                label="Password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                type={showPassword ? 'input' : 'password'}
                InputProps={{
                  endAdornment: (
                    <IconButton onClick={() => setShowPassword((show) => !show)}>
                      {showPassword ? <VisibilityOff /> : <Visibility />}
                    </IconButton>
                  ),
                }}
              />
              <PasswordStrength password={password} />
              <TextField
                label="Confirm password"
                value={confirmedPassword}
                onChange={(e) => setConfirmedPassword(e.target.value)}
                type={showConfirmedPassword ? 'input' : 'password'}
                InputProps={{
                  endAdornment: (
                    <IconButton onClick={() => setShowConfirmedPassword((show) => !show)}>
                      {showConfirmedPassword ? <VisibilityOff /> : <Visibility />}
                    </IconButton>
                  ),
                }}
              />
              <Button
                size="large"
                variant="contained"
                disabled={password !== confirmedPassword || password.length === 0}
              >
                Next
              </Button>
            </Stack>
          </FormControl>
        </Grid>
      </Grid>
    </>
  )
}
