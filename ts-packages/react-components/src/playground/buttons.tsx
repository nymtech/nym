import { Button, Stack } from '@mui/material';
import * as React from 'react';

export const PlaygroundButtons: React.FC = () => (
  <Stack spacing={2} direction="row">
    <Button variant="text">Text</Button>
    <Button variant="contained">Contained</Button>
    <Button variant="outlined">Outlined</Button>
  </Stack>
);
