import React from 'react';
import { ConnectionStatusKind } from '../types';

const getBusyFillColor = (color: string): string => {
  if (color === '#F4B02D') {
    return '#21D072';
  }
  return '#F4B02D';
};

const getStatusFillColor = (status: ConnectionStatusKind, hover: boolean, isError: boolean): string => {
  if (isError && hover) {
    return '#21D072';
  }
  if (isError) {
    return '#40475C';
  }

  switch (status) {
    case ConnectionStatusKind.disconnected:
      if (hover) {
        return '#FFFF33';
      }
      return '#FFE600';
    case ConnectionStatusKind.connecting:
    case ConnectionStatusKind.disconnecting:
      return '#FFE600';
    default:
      // connected
      if (hover) {
        return '#E43E3E';
      }
      return '#21D072';
  }
};

const getStatusText = (status: ConnectionStatusKind, hover: boolean): string => {
  switch (status) {
    case ConnectionStatusKind.disconnected:
      return 'Connect';
    case ConnectionStatusKind.connecting:
      return 'Connecting';
    case ConnectionStatusKind.disconnecting:
      return 'Connected';
    default:
      // connected
      if (hover) {
        return 'Disconnect';
      }
      return 'Connected';
  }
};

export const ConnectionButton: React.FC<{
  status: ConnectionStatusKind;
  disabled?: boolean;
  busy?: boolean;
  isError?: boolean;
  onClick?: (status: ConnectionStatusKind) => void;
}> = ({ status, disabled, isError, onClick, busy }) => {
  const [hover, setHover] = React.useState<boolean>(false);

  const handleClick = React.useCallback(() => {
    if (disabled === true) {
      return;
    }
    if (onClick) {
      onClick(status);
    }
  }, [status, disabled]);

  const statusText = getStatusText(status, hover);
  const statusTextColor = isError ? '#40475C' : '#FFF';
  const statusFillColor = getStatusFillColor(status, hover, Boolean(isError));

  return (
    <svg
      opacity={disabled ? 0.75 : 1}
      width="200"
      height="200"
      viewBox="0 0 200 200"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <g
        transform="translate(-27, -27)"
        onMouseEnter={() => !disabled && setHover(true)}
        onMouseLeave={() => !disabled && setHover(false)}
      >
        <g onClick={handleClick} style={{ cursor: disabled ? 'not-allowed' : 'pointer' }}>
          <g filter="url(#filter0_f_639_9730)">
            <circle cx="131" cy="131" r="51" fill="url(#paint0_radial_639_9730)" />
          </g>
          <circle cx="131" cy="131" r="65" fill="#1C1B1F" />
          <g filter="url(#filter1_d_639_9730)">
            <circle cx="131" cy="131" r="64" stroke={statusFillColor} strokeWidth="2" />
          </g>
          <circle cx="131" cy="131" r="73.5" stroke={statusFillColor} strokeOpacity="0.5" />
          {status === ConnectionStatusKind.connected && hover ? (
            <path
              d="M120.217 119.833C120.217 117.838 121.838 116.217 123.833 116.217H128.5V114H123.833C120.613 114 118 116.613 118 119.833C118 123.053 120.613 125.667 123.833 125.667H128.5V123.45H123.833C121.838 123.45 120.217 121.828 120.217 119.833ZM127 121H136.333V118.667H127V121ZM139.5 114H134.833V116.217H139.505C141.5 116.217 143.117 117.838 143.117 119.833C143.117 121.828 141.495 123.45 139.5 123.45H134.833V125.667H139.5C142.72 125.667 145.333 123.053 145.333 119.833C145.333 116.613 142.72 114 139.5 114Z"
              fill="white"
            />
          ) : (
            <path
              d="M122.217 119.833C122.217 117.838 123.838 116.217 125.833 116.217H130.5V114H125.833C122.613 114 120 116.613 120 119.833C120 123.053 122.613 125.667 125.833 125.667H130.5V123.45H125.833C123.838 123.45 122.217 121.828 122.217 119.833ZM127 121H136.333V118.667H127V121ZM137.5 114H132.833V116.217H137.505C139.5 116.217 141.117 117.838 141.117 119.833C141.117 121.828 139.495 123.45 137.5 123.45H132.833V125.667H137.5C140.72 125.667 143.333 123.053 143.333 119.833C143.333 116.613 140.72 114 137.5 114Z"
              fill="white"
            />
          )}
          <text
            className="button_text"
            x={131}
            y={146}
            fill={statusTextColor}
            dominantBaseline="middle"
            textAnchor="middle"
            fontWeight="700"
            fontSize="16px"
          >
            {statusText}
          </text>
          <defs>
            <filter
              id="filter0_f_639_9730"
              x="0"
              y="0"
              width="262"
              height="262"
              filterUnits="userSpaceOnUse"
              colorInterpolationFilters="sRGB"
            >
              <feFlood floodOpacity="0" result="BackgroundImageFix" />
              <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
              <feGaussianBlur stdDeviation="25" result="effect1_foregroundBlur_639_9730" />
            </filter>
            <filter
              id="filter1_d_639_9730"
              x="51"
              y="57"
              width="160"
              height="160"
              filterUnits="userSpaceOnUse"
              colorInterpolationFilters="sRGB"
            >
              <feFlood floodOpacity="0" result="BackgroundImageFix" />
              <feColorMatrix
                in="SourceAlpha"
                type="matrix"
                values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
                result="hardAlpha"
              />
              <feOffset dy="6" />
              <feGaussianBlur stdDeviation="7.5" />
              <feComposite in2="hardAlpha" operator="out" />
              <feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.3 0" />
              <feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_639_9730" />
              <feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_639_9730" result="shape" />
            </filter>
            <radialGradient
              id="paint0_radial_639_9730"
              cx="0"
              cy="0"
              r="1"
              gradientUnits="userSpaceOnUse"
              gradientTransform="translate(131 131) rotate(90) scale(51)"
            >
              <stop stopColor="#1C1C1F" />
              <stop offset="1" stopColor={statusFillColor} />
            </radialGradient>
          </defs>
        </g>
      </g>
    </svg>
  );
};
