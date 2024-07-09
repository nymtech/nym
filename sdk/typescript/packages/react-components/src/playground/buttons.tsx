import React from 'react';
import { Button, Stack } from '@mui/material';

export const PlaygroundButtons = () => (
  <Stack spacing={2} direction="row">
    <Button variant="text">Text</Button>
    <Button variant="contained">Contained</Button>
    <Button variant="outlined">Outlined</Button>
  </Stack>
);
