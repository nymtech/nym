import * as React from 'react';
import { useTheme } from '@mui/material/styles';

export const NetworkComponentsSVG: FCWithChildren = () => {
  const theme = useTheme();
  const color = theme.palette.nym.networkExplorer.nav.text;
  return (
    <svg width="25" height="25" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M17.2 10.5V4.40002L12 1.40002L6.8 4.40002V10.5L12 13.5L17.2 10.5Z"
        stroke={color}
        strokeMiterlimit="10"
      />
      <path d="M12 19.6V13.5L6.8 10.5L1.5 13.5V19.6L6.8 22.6L12 19.6Z" stroke={color} strokeMiterlimit="10" />
      <path d="M22.5 19.6V13.5L17.2 10.5L12 13.5V19.6L17.2 22.6L22.5 19.6Z" stroke={color} strokeMiterlimit="10" />
    </svg>
  );
};
