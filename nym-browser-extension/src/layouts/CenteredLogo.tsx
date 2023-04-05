import React from 'react';
import { Box } from '@mui/material';
import { LogoWithText } from 'src/components/LogoWithText';

const layoutStyle = {
  height: '100%',
  display: 'grid',
  gridTemplateColumns: '1fr',
  gridTemplateRows: 'repeat(3, 1fr)',
  gridColumnGap: '0px',
  gridRowGap: '0px',
};

export const CenteredLogoLayout = ({
  title,
  description,
  Actions,
}: {
  title: string;
  description?: string;
  Actions: React.ReactNode;
}) => (
  <Box style={layoutStyle}>
    <Box></Box>
    <LogoWithText title={title} description={description} />
    <Box sx={{ display: 'flex', justifyContent: 'flex-end' }}>{Actions}</Box>
  </Box>
);
