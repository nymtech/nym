import { Box, Container } from '@mui/material';
import React from 'react';

export const AppLayout = ({ children }: { children: React.ReactNode }) => (
  <Container maxWidth="xs" disableGutters sx={{ height: '100vh', p: 2 }}>
    {children}
  </Container>
);
