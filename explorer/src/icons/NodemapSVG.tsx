import * as React from 'react';
import { useTheme } from '@mui/material/styles';

export const NodemapSVG: FCWithChildren = () => {
  const theme = useTheme();
  const color = theme.palette.nym.networkExplorer.nav.text;
  return (
    <svg width="25" height="25" viewBox="0 0 19 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M1 9.6999C1 5.0999 4.7 1.3999 9.3 1.3999C13.9 1.3999 17.6 5.0999 17.6 9.6999C17.6 14.2999 9.3 21.5999 9.3 21.5999C9.3 21.5999 1 14.2999 1 9.6999Z"
        stroke={color}
        strokeMiterlimit="10"
      />
      <path
        d="M9.30005 12C11.233 12 12.8 10.433 12.8 8.5C12.8 6.567 11.233 5 9.30005 5C7.36705 5 5.80005 6.567 5.80005 8.5C5.80005 10.433 7.36705 12 9.30005 12Z"
        stroke={color}
        strokeMiterlimit="10"
      />
      <path d="M1.5 22.5999H17.1" stroke={color} strokeMiterlimit="10" strokeLinecap="round" />
    </svg>
  );
};
