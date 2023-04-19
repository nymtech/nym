import { Container } from '@mui/material';
import React from 'react';

export const AppLayout = ({ children }: { children: React.ReactNode }) => (
  <Container maxWidth="xs" disableGutters sx={{ height: '100vh' }}>
    {children}
  </Container>
);
