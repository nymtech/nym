import * as React from 'react';
import { useTheme } from '@mui/material/styles';

export const LightSwitchSVG: FCWithChildren = () => {
  const { palette } = useTheme();
  return (
    <svg width="26" height="26" viewBox="0 0 26 26" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M12 2C6.5 2 2 6.5 2 12C2 17.5 6.5 22 12 22C17.5 22 22 17.5 22 12C22 6.5 17.5 2 12 2Z"
        fill={palette.background.default}
      />
      <path d="M12 20C7.6 20 4 16.4 4 12C4 7.6 7.6 4 12 4V20Z" fill={palette.text.primary} />
    </svg>
  );
};
