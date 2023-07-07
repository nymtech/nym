import React from 'react';
import { Box } from '@mui/material';
import { BackButton, LogoWithText } from 'src/components/ui';

const layoutStyle = {
  height: '100%',
  display: 'grid',
  gridTemplateColumns: '1fr',
  gridTemplaterows: '1fr 2fr 1fr',
  gridColumnGap: '0px',
  gridRowGap: '0px',
  position: 'relative',
  p: 2,
};

export const TopLogoLayout = ({
  title,
  description,
  children,
  Actions,
}: {
  title: string;
  description?: string;
  children: React.ReactNode;
  Actions: React.ReactNode;
}) => (
  <Box sx={layoutStyle}>
    <Box sx={{ position: 'absolute', top: 16, left: 16 }}>
      <BackButton />
    </Box>
    <Box sx={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'center' }}>
      <LogoWithText logoSmall title={title} description={description} />
    </Box>
    <Box>{children}</Box>
    <Box sx={{ display: 'flex', alignItems: 'flex-end', width: '100%' }}>{Actions}</Box>
  </Box>
);
