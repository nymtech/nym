import * as React from 'react';
import { useTheme } from '@mui/material/styles';

export const GatewaysSVG: FCWithChildren = () => {
  const theme = useTheme();
  const color = theme.palette.text.primary;
  return (
    <svg width="24" height="24" viewBox="0 0 26 26" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M16.2 12H22.7" stroke={color} strokeWidth="1.3" strokeMiterlimit="10" strokeLinecap="round" />
      <path d="M1.30005 12H12" stroke={color} strokeWidth="1.3" strokeMiterlimit="10" strokeLinecap="round" />
      <path
        d="M20.1 9.40015L22.7 12.0001L20.1 14.6001"
        stroke={color}
        strokeWidth="1.3"
        strokeMiterlimit="10"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M13.2 22.7001H8.59998C6.89998 22.7001 5.59998 21.4001 5.59998 19.7001V4.30005C5.59998 2.60005 6.89998 1.30005 8.59998 1.30005H13.2C14.9 1.30005 16.2 2.60005 16.2 4.30005V19.6C16.2 21.3001 14.8 22.7001 13.2 22.7001Z"
        stroke={color}
        strokeWidth="1.3"
        strokeMiterlimit="10"
        strokeLinecap="round"
      />
    </svg>
  );
};
