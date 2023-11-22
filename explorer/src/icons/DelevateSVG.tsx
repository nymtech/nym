import * as React from 'react';
import { useTheme } from '@mui/material/styles';

export const DelegateSVG: FCWithChildren = () => {
  const theme = useTheme();
  const color = theme.palette.text.primary;

  return (
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M2.6665 7.99967V9.99967H3.99984V7.99967H2.6665ZM10.6665 4.66634L9.7265 3.72634L8.6665 4.77967V1.33301H7.33317V4.79301L6.25984 3.73967L5.33317 4.66634L7.99984 7.33301L10.6665 4.66634ZM2.6665 11.333H13.3332V9.99967H2.6665V11.333Z"
        fill="white"
      />
      <path
        d="M13.3332 13.6663C13.3332 14.2186 12.8855 14.6663 12.3332 14.6663H3.6665C3.11422 14.6663 2.6665 14.2186 2.6665 13.6663V13.333H13.3332V13.6663Z"
        fill="white"
      />
      <rect x="12" y="8" width="1.33333" height="2" fill="white" />
      <rect x="12" y="11.333" width="1.33333" height="2" fill="white" />
      <rect x="2.6665" y="11.333" width="1.33333" height="2" fill="white" />
    </svg>
  );
};
