import * as React from 'react';
import { useTheme } from '@mui/material/styles';

interface TelegramIconProps {
  size?: number | string;
  color?: string;
}

export const TelegramIcon: FCWithChildren<TelegramIconProps> = ({ size, color: colorProp }) => {
  const theme = useTheme();
  const color = colorProp || theme.palette.text.primary;
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none">
      <path
        d="M12.4 24C19.029 24 24.4 18.629 24.4 12C24.4 5.371 19.029 0 12.4 0C5.77102 0 0.400024 5.371 0.400024 12C0.400024 18.629 5.77102 24 12.4 24ZM5.89102 11.74L17.461 7.279C17.998 7.085 18.467 7.41 18.293 8.222L18.294 8.221L16.324 17.502C16.178 18.16 15.787 18.32 15.24 18.01L12.24 15.799L10.793 17.193C10.633 17.353 10.498 17.488 10.188 17.488L10.401 14.435L15.961 9.412C16.203 9.199 15.907 9.079 15.588 9.291L8.71702 13.617L5.75502 12.693C5.11202 12.489 5.09802 12.05 5.89102 11.74Z"
        fill={color}
      />
    </svg>
  );
};

TelegramIcon.defaultProps = {
  size: 24,
  color: undefined,
};
