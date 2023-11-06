import * as React from 'react';
import { useTheme } from '@mui/material/styles';

export const ElipsSVG: FCWithChildren = () => {
  const theme = useTheme();
  const color = theme.palette.text.primary;

  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="25" viewBox="0 0 24 25" fill="none">
      <circle cx="12" cy="12.5" r="12" fill="url(#paint0_angular_2549_7570)" />
      <defs>
        <radialGradient
          id="paint0_angular_2549_7570"
          cx="0"
          cy="0"
          r="1"
          gradientUnits="userSpaceOnUse"
          gradientTransform="translate(12 12.5) rotate(90) scale(12)"
        >
          <stop stop-color="#22D27E" />
          <stop offset="1" stop-color="#9002FF" />
        </radialGradient>
      </defs>
    </svg>
  );
};
