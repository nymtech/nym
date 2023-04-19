import React from 'react';
import { Button as MUIButton, ButtonProps } from '@mui/material';

export const Button = (props: ButtonProps) => (
  <MUIButton {...props} disableElevation sx={{ textTransform: 'initial' }} />
);
