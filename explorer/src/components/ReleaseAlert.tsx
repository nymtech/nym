import { Alert, Box, Typography } from '@mui/material';

export const ReleaseAlert = () => {
  return (
    <Alert severity="warning" sx={{ mb: 3, fontSize: 'medium', width: '100%' }}>
      <Box>
        <Typography>You are now viewing the legacy Nym mixnet explorer. Explorer 2.0 is coming soon.</Typography>
      </Box>
    </Alert>
  );
};
