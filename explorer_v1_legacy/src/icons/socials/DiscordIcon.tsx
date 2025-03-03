import * as React from 'react';
import { useTheme } from '@mui/material/styles';

interface DiscordIconProps {
  size?: number | string;
  color?: string;
}

export const DiscordIcon: FCWithChildren<DiscordIconProps> = ({ size, color: colorProp }) => {
  const theme = useTheme();
  const color = colorProp || theme.palette.text.primary;
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none">
      <g clipPath="url(#clip0_1223_2296)">
        <path
          d="M12.4 0C5.80002 0 0.400024 5.4 0.400024 12C0.400024 18.6 5.80002 24 12.4 24C19 24 24.4 18.6 24.4 12C24.4 5.4 19 0 12.4 0ZM20.1 15.9C18.8 16.9 17.5 17.5 16.2 17.9C16.2 17.9 16.2 17.9 16.1 17.9C15.8 17.5 15.5 17.1 15.3 16.6V16.5C15.7 16.3 16.1 16.1 16.5 15.9V15.8C16.4 15.7 16.3 15.7 16.3 15.6C16.3 15.6 16.3 15.6 16.2 15.6C13.7 16.8 10.9 16.8 8.40002 15.6C8.40002 15.6 8.40002 15.6 8.30002 15.6C8.20002 15.7 8.10002 15.7 8.10002 15.8V15.9C8.50002 16.1 8.90002 16.3 9.30002 16.5C9.30002 16.5 9.30002 16.5 9.30002 16.6C9.10002 17.1 8.80002 17.5 8.50002 17.9C8.50002 17.9 8.50002 17.9 8.40002 17.9C7.10002 17.5 5.90002 16.9 4.50002 15.9C4.40002 13 5.00002 10.1 7.00002 7.1C8.00002 6.6 9.00002 6.3 10.2 6.1C10.2 6.1 10.2 6.1 10.3 6.1C10.4 6.3 10.6 6.7 10.7 6.9C11.9 6.7 13.1 6.7 14.2 6.9C14.3 6.7 14.5 6.3 14.6 6.1C14.6 6.1 14.6 6.1 14.7 6.1C15.8 6.3 16.9 6.6 17.9 7.1C19.5 9.7 20.4 12.6 20.1 15.9Z"
          fill={color}
        />
        <path
          d="M15 11C14.2 11 13.6 11.7 13.6 12.6C13.6 13.5 14.2 14.2 15 14.2C15.8 14.2 16.4 13.5 16.4 12.6C16.4 11.7 15.8 11 15 11Z"
          fill={color}
        />
        <path
          d="M9.80002 11C9.10002 11 8.40002 11.7 8.40002 12.6C8.40002 13.5 9.00002 14.2 9.80002 14.2C10.6 14.2 11.2 13.5 11.2 12.6C11.2 11.7 10.6 11 9.80002 11Z"
          fill={color}
        />
      </g>
      <defs>
        <clipPath id="clip0_1223_2296">
          <rect width="24" height="24" transform="translate(0.400024)" />
        </clipPath>
      </defs>
    </svg>
  );
};

DiscordIcon.defaultProps = {
  size: 24,
  color: undefined,
};
