import React from 'react';
import { Box } from '@mui/material';
import { Button } from 'src/components/ui';
import { CenteredLogoLayout } from 'src/layouts';

export const SetupComplete = () => (
  <CenteredLogoLayout
    title="You're all set!"
    description="Open the extension and sign in to begin your interchain journey"
    Actions={
      <Box width="100%">
        <Button variant="contained" fullWidth size="large">
          Done
        </Button>
      </Box>
    }
  />
);
