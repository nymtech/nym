import * as React from 'react';
import { useTheme } from '@mui/material/styles';

export const ValidatorsSVG: FCWithChildren = () => {
  const theme = useTheme();
  const color = theme.palette.text.primary;
  return (
    <svg width="24" height="24" viewBox="0 0 26 26" fill="none" xmlns="http://www.w3.org/2000/svg">
      <g clipPath="url(#clip0)">
        <path
          d="M18.2001 18.4V19.7001C18.2001 21.4001 16.9 22.7001 15.2 22.7001H4.30005C2.60005 22.7001 1.30005 21.4001 1.30005 19.7001V4.30005C1.30005 2.60005 2.60005 1.30005 4.30005 1.30005H15.1C16.8 1.30005 18.1 2.60005 18.1 4.30005V5.60005V18.4H18.2001Z"
          stroke={color}
          strokeWidth="1.3"
          strokeMiterlimit="10"
          strokeLinecap="round"
        />
        <path
          d="M13.4 22.7001H17.4C19.1 22.7001 20.4 21.4001 20.4 19.7001V18.4V5.60005V4.30005C20.4 2.60005 19.1 1.30005 17.4 1.30005H11.5"
          stroke={color}
          strokeWidth="1.3"
          strokeMiterlimit="10"
          strokeLinecap="round"
        />
        <path
          d="M15.2 22.7001H19.7C21.4 22.7001 22.7 21.4001 22.7 19.7001V18.4V5.60005V4.30005C22.7 2.60005 21.4 1.30005 19.7 1.30005H13.8"
          stroke={color}
          strokeWidth="1.3"
          strokeMiterlimit="10"
          strokeLinecap="round"
        />
        <path
          d="M5 12.3L7.9 15.3L14.5 8.69995"
          stroke={color}
          strokeWidth="2"
          strokeMiterlimit="10"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </g>
      <defs>
        <clipPath id="clip0">
          <rect width="24" height="24" fill="white" />
        </clipPath>
      </defs>
    </svg>
  );
};
