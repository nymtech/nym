import React from 'react';
import { Stack } from '@mui/system';
import { Button } from 'src/components/ui';
import { CenteredLogoLayout } from 'src/layouts';
import { Link } from 'react-router-dom';

export const Home = () => {
  return (
    <CenteredLogoLayout
      title="Welcome to Nym"
      Actions={
        <Stack gap={2} width="100%" justifyContent="flex-end">
          <Link to="/register" style={{ textDecoration: 'none' }}>
            <Button variant="contained" disableElevation size="large" fullWidth>
              Create new account
            </Button>
          </Link>
          <Button variant="text" disableElevation size="large" fullWidth color="primary">
            Import existing account
          </Button>
        </Stack>
      }
    />
  );
};
