import React, { useContext } from 'react';
import { Box, Button, Divider, Stack, TextField, Typography } from '@mui/material';
import { AppContext } from '../../context/main';

export const Profile = () => {
  const { mixnodeDetails } = useContext(AppContext);

  if (!mixnodeDetails) return null;

  return (
    <>
      <Box sx={{ p: 3 }}>
        <Stack spacing={3}>
          <Typography sx={{ color: (theme) => theme.palette.text.disabled }}>
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
  );
};
