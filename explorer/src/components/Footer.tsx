import * as React from 'react';
import Box from '@mui/material/Box';
import { Typography, useMediaQuery } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { Socials } from './Socials';

export const Footer: React.FC = () => {
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));

  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center',
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
          <Socials isFooter />
        </Box>
      )}
      <Typography
        sx={{
          fontSize: 12,
          textAlign: isMobile ? 'center' : 'end',
          color: theme.palette.nym.text.footer,
        }}
      >
        © {new Date().getFullYear()} Nym Technologies SA, all rights reserved
      </Typography>
    </Box>
  );
};
