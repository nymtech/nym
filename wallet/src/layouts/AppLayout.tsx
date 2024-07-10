import { useContext } from 'react';
import { Box, Container, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { AppContext } from 'src/context';
import { AppBar, LoadingPage, Nav } from '../components';
// import { NymWordmark } from '@nymproject/react';

export const ApplicationLayout: FCWithChildren = ({ children }) => {
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
            py: 4,
          }}
          display="flex"
          flexDirection="column"
          justifyContent="space-between"
        >
          <Box>
            <Box sx={{ ml: 5, mb: 7 }}>{/* <NymWordmark height={14} /> */}</Box>
            <Nav />
          </Box>
          {appVersion && (
            <Typography sx={{ color: 'grey.500', fontSize: 14, ml: 5, mt: 8 }}>Version {appVersion}</Typography>
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
