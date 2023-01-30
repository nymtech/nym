import React from 'react';
import { ConnectionStatusKind } from 'src/types';

const getStatusFillColor = (status: ConnectionStatusKind, hover: boolean, isError: boolean): string => {
  if (isError && hover) {
    return '#21D072';
  }
  if (isError) {
    return '#40475C';
  }

  switch (status) {
    case 'disconnected':
      if (hover) {
        return '#FFF';
      }
      return '#BBB';
    case 'connecting':
      return '#FFF';
    case 'disconnecting':
      return '#FFF';
    default:
      // connected
      if (hover) {
        return '#E43E3E';
      }
      return '#21D072';
  }
};

export const PowerButton: FCWithChildren<{
  onClick?: (status: ConnectionStatusKind) => void;
  isError?: boolean;
  disabled?: boolean;
  status: ConnectionStatusKind;
  busy?: boolean;
}> = ({ onClick, disabled, status, isError }) => {
  const [hover, setHover] = React.useState<boolean>(false);

  const handleClick = () => {
    if (disabled === true) {
      return;
    }
    if (onClick) {
      onClick(status);
    }
  };

  const statusFillColor = getStatusFillColor(status, hover, Boolean(isError));

  return (
    <svg
      width="190"
      height="190"
      viewBox="0 0 200 200"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      onClick={handleClick}
      style={{ cursor: disabled ? 'not-allowed' : 'pointer' }}
      onMouseEnter={() => !disabled && setHover(true)}
      onMouseLeave={() => !disabled && setHover(false)}
    >
      <g transform="translate(-30, -25) ">
        <circle cx={131} cy={131} r={70} strokeWidth={2} stroke={statusFillColor} filter="url(#blur)" opacity="0.6" />
        <circle cx={131} cy={131} r={22} strokeWidth={1} stroke={statusFillColor} filter="url(#blur)" opacity="0.3" />
        <circle opacity={0.6} cx={131} cy={131} r={68.5} stroke={statusFillColor} />
        <g filter="url(#filter1_d_944_9033)">
          <circle cx={131} cy={131} r={64} fill="url(#paint1_radial_944_9033)" />
          <circle cx={131} cy={131} r={63} stroke={statusFillColor} strokeWidth={2} />
        </g>
        <g opacity={0.5} filter="url(#filter2_f_944_9033)">
          <g clipPath="url(#clip0_944_9033)">
            <path
              d="M131 113C129.9 113 129 113.9 129 115V131C129 132.1 129.9 133 131 133C132.1 133 133 132.1 133 131V115C133 113.9 132.1 113 131 113ZM141.28 118.72C140.5 119.5 140.52 120.72 141.26 121.5C143.52 123.9 144.92 127.1 145 130.64C145.18 138.3 138.84 144.9 131.18 144.98C123.36 145.1 117 138.8 117 131C117 127.32 118.42 123.98 120.74 121.48C121.48 120.7 121.48 119.48 120.72 118.72C119.92 117.92 118.62 117.94 117.86 118.76C114.96 121.84 113.14 125.94 113 130.48C112.72 140.24 120.66 148.68 130.42 148.98C140.62 149.3 149 141.12 149 130.98C149 126.24 147.16 121.96 144.16 118.76C143.4 117.94 142.08 117.92 141.28 118.72Z"
              stroke={statusFillColor}
            />
          </g>
        </g>
        <g clipPath="url(#clip1_944_9033)">
          <path
            d="M131 113C129.9 113 129 113.9 129 115V131C129 132.1 129.9 133 131 133C132.1 133 133 132.1 133 131V115C133 113.9 132.1 113 131 113ZM141.28 118.72C140.5 119.5 140.52 120.72 141.26 121.5C143.52 123.9 144.92 127.1 145 130.64C145.18 138.3 138.84 144.9 131.18 144.98C123.36 145.1 117 138.8 117 131C117 127.32 118.42 123.98 120.74 121.48C121.48 120.7 121.48 119.48 120.72 118.72C119.92 117.92 118.62 117.94 117.86 118.76C114.96 121.84 113.14 125.94 113 130.48C112.72 140.24 120.66 148.68 130.42 148.98C140.62 149.3 149 141.12 149 130.98C149 126.24 147.16 121.96 144.16 118.76C143.4 117.94 142.08 117.92 141.28 118.72Z"
            fill={statusFillColor}
          />
        </g>
        <defs>
          <filter
            id="filter0_f_944_9033"
            x={0}
            y={0}
            width={240}
            height={240}
            filterUnits="userSpaceOnUse"
            colorInterpolationFilters="sRGB"
          >
            <feFlood floodOpacity={0} result="BackgroundImageFix" />
            <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
            <feGaussianBlur stdDeviation={40} result="effect1_foregroundBlur_944_9033" />
          </filter>
          <filter
            id="filter1_d_944_9033"
            x={52}
            y={58}
            width={158}
            height={158}
            filterUnits="userSpaceOnUse"
            colorInterpolationFilters="sRGB"
          >
            <feFlood floodOpacity={0} result="BackgroundImageFix" />
            <feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_944_9033" />
            <feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_944_9033" result="shape" />
          </filter>
          <filter
            id="filter2_f_944_9033"
            x={97}
            y={97}
            width={68}
            height={68}
            filterUnits="userSpaceOnUse"
            colorInterpolationFilters="sRGB"
          >
            <feFlood floodOpacity={0} result="BackgroundImageFix" />
            <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
            <feGaussianBlur stdDeviation={5} result="effect1_foregroundBlur_944_9033" />
          </filter>
          <filter id="blur">
            <feGaussianBlur stdDeviation="5" />
          </filter>
        </defs>
      </g>
    </svg>
  );
};
