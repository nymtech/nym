import React from 'react';
import { NymWordmark } from '@nymproject/react';
import { Box, Container } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { AppBar, Nav } from '../components';

export const ApplicationLayout: React.FC = ({ children }) => {
  const theme = useTheme();
  return (
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
          <Box sx={{ mb: 4 }}>
            <NymWordmark height={14} fill={theme.palette.background.paper} />
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
};
