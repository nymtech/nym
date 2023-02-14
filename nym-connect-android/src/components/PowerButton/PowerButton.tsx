import React, { useCallback } from 'react';
import { ConnectionStatusKind } from 'src/types';
import './power-button.css';

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

  const getClassName = useCallback(() => {
    if (hover) {
      switch (status) {
        case 'disconnected':
          return 'expand';
        default:
          return 'contract';
      }
    }
    if (!hover) {
      switch (status) {
        case 'connected':
          return 'expand';
        default:
          return 'contract';
      }
    }
    return 'contract';
  }, [status, hover]);

  const buttonPulse = () => {
    if (status === 'connecting' || status === 'disconnecting') return 'pulse';
    return undefined;
  };

  let statusText: string;
  if (status === 'connected') {
    statusText = 'stop';
  } else {
    statusText = status === 'disconnected' ? 'start' : '';
  }

  return (
    <svg
      width="220"
      height="220"
      viewBox="0 0 200 200"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      onClick={handleClick}
      style={{ cursor: disabled ? 'not-allowed' : 'pointer' }}
      onMouseEnter={() => !disabled && setHover(true)}
      onMouseLeave={() => !disabled && setHover(false)}
    >
      <g transform="translate(-30, -25) ">
        <circle cx={131} cy={131} r={75} strokeWidth={4} stroke={statusFillColor} filter="url(#blur)" opacity="0.6" />
        <circle cx={131} cy={131} r={25} strokeWidth={2} stroke={statusFillColor} filter="url(#blur)" opacity="0.5" />
        <g id="Button power">
          <circle cx="131" cy="131" r="68.5" stroke={statusFillColor} strokeWidth="0.5" />
          <circle id="ring-one" className={getClassName()} cx="131" cy="131" r="73" stroke={statusFillColor} />
          <circle id="ring-two" className={getClassName()} cx="131" cy="131" r="77" stroke={statusFillColor} />
          <circle id="ring-three" className={getClassName()} cx="131" cy="131" r="81" stroke={statusFillColor} />
          <circle id="ring-four" className={getClassName()} cx="131" cy="131" r="85" stroke={statusFillColor} />
          <g id="button bg">
            <circle cx="131" cy="131" r="63" stroke={statusFillColor} strokeWidth="3" className={buttonPulse()} />
          </g>
          <g id="Power icon">
            <g id="Icon">
              <g id="Group 672_2">
                <g id="power_settings_new_black_24dp (1) 1_2" clipPath="url(#clip1_944_8739)">
                  <path
                    id="Vector_2"
                    d="M131 113C129.9 113 129 113.9 129 115V131C129 132.1 129.9 133 131 133C132.1 133 133 132.1 133 131V115C133 113.9 132.1 113 131 113ZM141.28 118.72C140.5 119.5 140.52 120.72 141.26 121.5C143.52 123.9 144.92 127.1 145 130.64C145.18 138.3 138.84 144.9 131.18 144.98C123.36 145.1 117 138.8 117 131C117 127.32 118.42 123.98 120.74 121.48C121.48 120.7 121.48 119.48 120.72 118.72C119.92 117.92 118.62 117.94 117.86 118.76C114.96 121.84 113.14 125.94 113 130.48C112.72 140.24 120.66 148.68 130.42 148.98C140.62 149.3 149 141.12 149 130.98C149 126.24 147.16 121.96 144.16 118.76C143.4 117.94 142.08 117.92 141.28 118.72Z"
                    fill={statusFillColor}
                    className={buttonPulse()}
                  />
                </g>
              </g>
            </g>
          </g>
          <text
            className="button_text"
            x={131}
            y={165}
            fill={statusFillColor}
            dominantBaseline="middle"
            textAnchor="middle"
            fontWeight="500"
            fontSize="12px"
            letterSpacing={1}
          >
            {statusText.toUpperCase()}
          </text>
        </g>
        <defs>
          <filter id="blur" width="200%" height="200%" x="-50%" y="-50%">
            <feGaussianBlur stdDeviation="12.5" />
          </filter>
        </defs>
      </g>
    </svg>
  );
};
