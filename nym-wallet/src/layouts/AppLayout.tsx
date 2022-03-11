import React from 'react';
import { Box, Container } from '@mui/material';
import { AppBar, Nav } from '../components';
import { NymLogo } from '@nymproject/react';

export const ApplicationLayout: React.FC = ({ children }) => (
  <Box
    sx={{
      height: '100vh',
      width: '100vw',
      display: 'grid',
      gridTemplateColumns: '240px auto',
      gridTemplateRows: '100%',
      overflow: 'hidden',
    }}
  >
    <Box
      sx={{
        background: '#121726',
        overflow: 'auto',
        py: 4,
        px: 5,
      }}
      display="flex"
      flexDirection="column"
      justifyContent="space-between"
    >
      <Box>
        <Box sx={{ mb: 3 }}>
          <NymLogo width={45} />
        </Box>
        <Nav />
      </Box>
    </Box>
    <Container>
      <AppBar />
      {children}
    </Container>
  </Box>
);
