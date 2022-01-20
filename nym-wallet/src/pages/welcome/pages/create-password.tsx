import React, { useState } from 'react'
import { FormControl, Grid, Stack, TextField } from '@mui/material'
import { Subtitle, Title, PasswordStrength } from '../components'

export const CreatePassword = ({}: { page: 'create password' }) => {
  const [password, setPassword] = useState<string>("")
  return (
    <>
      <Title title="Create password" />
      <Subtitle subtitle="Create strong password, min 8 characters, at least one capital letter, number and special sign" />
      <Grid container justifyContent="center">
        <Grid item xs={6}>
          <FormControl fullWidth>
            <Stack spacing={2}>
              <TextField label="Password" value={password} onChange={(e) => setPassword(e.target.value)} type="password"/>
              <PasswordStrength password={password}/>
              <TextField label="Confirm password" />
            </Stack>
          </FormControl>
        </Grid>
      </Grid>
    </>
  )
}
