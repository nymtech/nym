import React from 'react';
import { Box, Button, Typography } from '@mui/material';
import HelpOutlineIcon from '@mui/icons-material/HelpOutline';

const HELP_URL = 'https://docs.nymtech.net';

export const NeedHelp: React.FC = () => (
  <Box sx={{ display: 'grid', placeItems: 'center' }}>
    <Button component="a" href={HELP_URL} target="_blank" sx={{ fontSize: '12px', fontWeight: '600' }} color="info">
      <HelpOutlineIcon color="inherit" fontSize="inherit" fontWeight="inherit" />
      <Typography ml={0.5} color="inherit" fontSize="inherit" fontWeight="inherit">
        Need help?
      </Typography>
    </Button>
  </Box>
);
