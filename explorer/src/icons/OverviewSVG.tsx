import * as React from 'react';
import { useTheme } from '@mui/material/styles';

export const OverviewSVG: FCWithChildren = () => {
  const theme = useTheme();
  const color = theme.palette.nym.networkExplorer.nav.text;

  return (
    <svg width="25" height="25" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M1.4 21.6H22.6" stroke={color} strokeMiterlimit="10" strokeLinecap="round" />
      <path d="M14.1 2.40002H9.9V21.5H14.1V2.40002Z" stroke={color} strokeMiterlimit="10" strokeLinecap="round" />
      <path d="M20.8 6.59998H16.6V21.5H20.8V6.59998Z" stroke={color} strokeMiterlimit="10" strokeLinecap="round" />
      <path d="M7.4 11.8H3.2V21.6H7.4V11.8Z" stroke={color} strokeMiterlimit="10" strokeLinecap="round" />
    </svg>
  );
};
