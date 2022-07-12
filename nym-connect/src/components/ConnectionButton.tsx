import React from 'react';
import { ConnectionStatusKind } from '../types';

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
      width="208"
      height="208"
      viewBox="0 0 208 208"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      onMouseEnter={() => !disabled && setHover(true)}
      onMouseLeave={() => !disabled && setHover(false)}
    >
      <g transform="translate(-46 -46)">
        <g onClick={handleClick} style={{ cursor: disabled ? 'not-allowed' : 'pointer' }}>
          <g filter="url(#filter0_f_2_303)">
            <circle cx="150" cy="150" r="70" fill="#3B445F" />
          </g>
          <g filter="url(#filter1_d_2_303)">
            <circle cx="150" cy="150" r="65" fill="url(#paint0_radial_2_303)" />
            <circle cx="150" cy="150" r="61.5" stroke={statusFillColor} strokeWidth="7">
              {busy && (
                <animate
                  attributeName="stroke"
                  values={`${statusFillColor};${getBusyFillColor(statusFillColor)};${statusFillColor}`}
                  dur="1.5s"
                  repeatCount="indefinite"
                />
              )}
            </circle>
          </g>
          <circle cx="150" cy="150" r="73.5" stroke="white" strokeOpacity="0.2" />
          {status === ConnectionStatusKind.connected && hover ? (
            <path
              d="M136.264 135.833C136.264 133.838 137.92 132.217 139.957 132.217H144.723V130H139.957C136.669 130 134 132.613 134 135.833C134 139.053 136.669 141.667 139.957 141.667H144.723V139.45H139.957C137.92 139.45 136.264 137.828 136.264 135.833ZM145.234 137H154.766V134.667H145.234V137ZM160.043 130H155.277V132.217H160.043C162.08 132.217 163.736 133.838 163.736 135.833C163.736 137.828 162.08 139.45 160.043 139.45H155.277V141.667H160.043C163.331 141.667 166 139.053 166 135.833C166 132.613 163.331 130 160.043 130Z"
              fill="white"
            />
          ) : (
            <path
              d="M140.217 135.833C140.217 133.838 141.838 132.217 143.833 132.217H148.5V130H143.833C140.613 130 138 132.613 138 135.833C138 139.053 140.613 141.667 143.833 141.667H148.5V139.45H143.833C141.838 139.45 140.217 137.828 140.217 135.833ZM145 137H154.333V134.667H145V137ZM155.5 130H150.833V132.217H155.505C157.5 132.217 159.117 133.838 159.117 135.833C159.117 137.828 157.495 139.45 155.5 139.45H150.833V141.667H155.5C158.72 141.667 161.333 139.053 161.333 135.833C161.333 132.613 158.72 130 155.5 130Z"
              fill="white"
            />
          )}
          <text
            className="button_text"
            x={150}
            y={160}
            fill={statusTextColor}
            dominantBaseline="middle"
            textAnchor="middle"
            fontWeight="700"
            fontSize="14px"
          >
            {statusText}
          </text>
          <defs>
            <filter
              id="filter0_f_2_303"
              x="0"
              y="0"
              width="300"
              height="300"
              filterUnits="userSpaceOnUse"
              colorInterpolationFilters="sRGB"
            >
              <feFlood floodOpacity="0" result="BackgroundImageFix" />
              <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
              <feGaussianBlur stdDeviation="20" result="effect1_foregroundBlur_2_303" />
            </filter>
            <filter
              id="filter1_d_2_303"
              x="70"
              y="76"
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
              <feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_2_303" />
              <feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_2_303" result="shape" />
            </filter>
            <radialGradient
              id="paint0_radial_2_303"
              cx="0"
              cy="0"
              r="1"
              gradientUnits="userSpaceOnUse"
              gradientTransform="translate(150 150) rotate(90) scale(65)"
            >
              <stop stopColor="#283046" />
              <stop offset="1" stopColor="#121727" />
            </radialGradient>
          </defs>
        </g>
      </g>
    </svg>
  );
};

const getBusyFillColor = (color: string): string => {
  if (color === '#60D6EF') {
    return '#21D072';
  }
  return '#60D6EF';
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
        return '#21D072';
      }
      return '#60D6EF';
    case ConnectionStatusKind.connecting:
    case ConnectionStatusKind.disconnecting:
      return '#60D6EF';
    default:
      // connected
      if (hover) {
        return '#DA465B';
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
