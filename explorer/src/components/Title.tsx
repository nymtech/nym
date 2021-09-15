import React from 'react';
import { Box, Grid, Typography } from '@mui/material';

export interface TitleProps {
  text: string;
}

export const Title = ({ text }: TitleProps) => (
  <Grid
    item
    xs={12}
    sx={{
      justifyContent: 'flex-start',
      padding: (theme) => theme.spacing(2),
      backgroundColor: (theme) => theme.palette.primary.dark,
    }}
  >
    <Box
      sx={{
        padding: (theme) => theme.spacing(3),
        backgroundColor: (theme) => theme.palette.primary.light,
      }}
    >
      <Typography
        sx={{
          color: (theme) => theme.palette.primary.main,
        }}
      >
        {text}
      </Typography>
    </Box>
  </Grid>
);
