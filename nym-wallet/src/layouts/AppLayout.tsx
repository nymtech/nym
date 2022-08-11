import React, { useContext } from 'react';
import { NymWordmark } from '@nymproject/react/logo/NymWordmark';
import { Box, Container, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { AppContext } from 'src/context';
import { AppBar, LoadingPage, Nav } from '../components';

export const ApplicationLayout: React.FC = ({ children }) => {
  const theme = useTheme();
  const { isLoading, appVersion } = useContext(AppContext);

  return (
    <>
      {isLoading && <LoadingPage />}
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
            background: (t) => t.palette.nym.nymWallet.nav.background,
            overflow: 'auto',
            py: 5,
          }}
          display="flex"
          flexDirection="column"
          justifyContent="space-between"
        >
          <Box>
            <Box sx={{ ml: 5, mb: 3 }}>
              <NymWordmark height={14} />
            </Box>
            <Nav />
          </Box>
          {appVersion && (
            <Typography sx={{ color: (t) => t.palette.grey[500], fontSize: 14, ml: 5, mt: 8 }}>
              Version {appVersion}
            </Typography>
          )}
        </Box>
        <Container maxWidth="xl">
          <AppBar />
          <Box overflow="auto" sx={{ height: () => `calc(100% - ${theme.spacing(10)})` }}>
            {children}
          </Box>
        </Container>
      </Box>
    </>
  );
};
