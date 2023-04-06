import React from 'react';
import { Box, IconButton } from '@mui/material';
import { LogoWithText } from 'src/components/ui';
import { ArrowBackIosRounded } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

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
}) => {
  const navigate = useNavigate();
  return (
    <Box sx={layoutStyle}>
      <Box sx={{ position: 'absolute', top: 16, left: 16 }}>
        <IconButton size="small" onClick={() => navigate(-1)}>
          <ArrowBackIosRounded fontSize="small" />
        </IconButton>
      </Box>

      <Box sx={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'center' }}>
        <LogoWithText logoSmall title={title} description={description} />
      </Box>
      <Box>{children}</Box>
      <Box sx={{ display: 'flex', alignItems: 'flex-end', width: '100%' }}>{Actions}</Box>
    </Box>
  );
};
