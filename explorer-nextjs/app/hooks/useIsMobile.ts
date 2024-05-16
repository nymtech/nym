'use client'

import { Breakpoint, useMediaQuery } from '@mui/material';
import { useTheme } from '@mui/material/styles';

export const useIsMobile = (queryInput: number | Breakpoint = 'md') => {
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down(queryInput));

  return isMobile;
};
