import React from 'react';
import { Box } from '@mui/material';
import { Button } from 'src/components/ui';
import { CenteredLogoLayout } from 'src/layouts';

export const SetupCompleteTemplate = ({
  title,
  description,
  onDone,
}: {
  title: string;
  description: string;
  onDone: () => void;
}) => (
  <CenteredLogoLayout
    title={title}
    description={description}
    Actions={
      <Box width="100%">
        <Button variant="contained" fullWidth size="large" onClick={onDone}>
          Done
        </Button>
      </Box>
    }
  />
);
