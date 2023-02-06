import React from 'react';
import { Box } from '@mui/material';

export const HelpImage = ({ img, imageDescription }: { img: string; imageDescription: string }) => (
  <img src={img} alt={imageDescription} width="100%" />
);
