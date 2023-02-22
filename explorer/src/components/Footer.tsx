import * as React from 'react';
import Box from '@mui/material/Box';
import { Typography } from '@mui/material';
import { Socials } from './Socials';
import { useIsMobile } from '../hooks/useIsMobile';

export const Footer: React.FC = () => {
  const isMobile = useIsMobile();

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
          color: 'nym.muted.onDarkBg',
        }}
      >
        © {new Date().getFullYear()} Nym Technologies SA, all rights reserved
      </Typography>
    </Box>
  );
};
