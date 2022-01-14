import { Button, Stack, TextField } from '@mui/material'
import { Box } from '@mui/system'
import React from 'react'
import { InfoTooltip } from '../../components/InfoToolTip'

export const Profile = () => {
  return (
    <>
      <Box sx={{ p: 3 }}>
        <Stack spacing={3}>
          <TextField label="Mixnode name" disabled />
          <TextField multiline label="Mixnode description" rows={3} disabled />
          <TextField label="Link" disabled />
        </Stack>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          padding: 3,
        }}
      >
        <Button variant="contained" size="large" color="primary" type="submit" disableElevation disabled>
          Update
        </Button>
      </Box>
    </>
  )
}
