import { Alert, Box, Link, Typography } from '@mui/material';
import StyledLink from './StyledLink';

export const ReleaseAlert = () => (
  <Alert severity="warning" sx={{ mb: 3, fontSize: 'medium', width: '100%' }}>
    <Box>
      <Typography>
        "You are now viewing the legacy Nym mixnet explorer. Check out Explorer 2.0 here link
        <Link
            href="https://nym.com/explorer"
            style={{ textDecoration: 'none' }}
        />
      </Typography>
    </Box>
  </Alert>
);
