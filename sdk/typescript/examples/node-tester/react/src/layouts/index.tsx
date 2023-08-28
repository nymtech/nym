import React from 'react';
import { Container } from '@mui/material';

export const BasicPageLayout = ({ children }: { children: React.ReactNode }) => (
  <Container maxWidth="md">{children}</Container>
);
