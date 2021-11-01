import React from 'react';
import { Typography, useTheme } from '@mui/material';
import { MainContext } from 'src/context/main';

export const Title: React.FC<{ text: string }> = ({ text }) => {
  const { mode } = React.useContext(MainContext);
  const theme = useTheme();
  const color =
    mode === 'dark'
      ? theme.palette.primary.light
      : theme.palette.secondary.main;
  return (
    <Typography
      variant="h5"
      sx={{
        color,
        mb: 3,
      }}
    >
      {text}
    </Typography>
  );
};
