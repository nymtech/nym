import * as React from 'react';
import Box from '@mui/material/Box';
import { Typography, useMediaQuery, useTheme } from '@mui/material';
import { MainContext } from 'src/context/main';
import { palette } from '../index';
import { Socials } from './Socials';

export const Footer: React.FC = () => {
  const { mode } = React.useContext(MainContext);
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('sm'));

  return (
    <>
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          background: mode === 'dark' ? palette.blackBg : palette.primary.main,
          width: '100%',
          height: 'auto',
          mt: 3,
          pt: 3,
          pb: 3,
        }}
      >
        {isMobile && (
          <Box
            sx={{
              display: 'flex',
              flexDirection: 'row',
              width: 'auto',
              justifyContent: 'center',
              alignItems: 'center',
              mb: 2,
            }}
          >
            <Socials />
          </Box>
        )}
        <Typography
          sx={{
            fontSize: 12,
            textAlign: isMobile ? 'center' : 'end',
          }}
        >
          © 2021 Nym Technologies SA, all rights reserved
        </Typography>
      </Box>
    </>
  );
};
