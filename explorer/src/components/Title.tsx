import React from 'react';
import { Box, Grid, Typography } from '@mui/material';

export const Title: React.FC<{ text: string }> = ({ text }) => (
  <Grid
    item
    xs={12}
    sx={{
      justifyContent: 'flex-start',
      padding: 2,
      bgcolor: 'primary.dark',
    }}
  >
    <Box
      sx={{
        padding: 3,
        bgcolor: 'primary.light',
      }}
    >
      <Typography sx={{ color: 'primary.main' }}>{text}</Typography>
    </Box>
  </Grid>
);
