import React, { useContext } from 'react'
import { Button, Divider, Stack, TextField, Typography } from '@mui/material'
import { Box } from '@mui/system'
import { ClientContext } from '../../context/main'

export const Profile = () => {
  const { mixnodeDetails } = useContext(ClientContext)
  return (
    <>
      <Box sx={{ p: 3 }}>
        <Stack spacing={3}>
          <Typography sx={{ color: 'grey.600' }}>
            Node identity: {mixnodeDetails?.mix_node.identity_key || 'n/a'}
          </Typography>
          <Divider />
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
