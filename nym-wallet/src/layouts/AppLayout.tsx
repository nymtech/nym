import React, { useContext } from 'react';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
import { Box, Container } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { AppContext } from 'src/context';
import { Settings } from 'src/pages';
import { AppBar, LoadingPage, Nav } from '../components';

export const ApplicationLayout: React.FC = ({ children }) => {
  const theme = useTheme();
  const { isLoading, showSettings } = useContext(AppContext);

  return (
    <>
      {isLoading && <LoadingPage />}
      {showSettings && <Settings />}
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
            py: 3,
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
        <Container maxWidth="xl">
          <AppBar />
          {children}
        </Container>
      </Box>
    </>
  );
};
