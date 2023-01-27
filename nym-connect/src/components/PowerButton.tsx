import React from 'react';
import { ConnectionStatusKind } from 'src/types';

export const PowerButton: FCWithChildren<{
  onClick: (status: ConnectionStatusKind) => void;
  isError?: boolean;
  disabled: boolean;
  status: ConnectionStatusKind;
  busy?: boolean;
}> = ({ onClick, disabled, status }) => {
  const [hover, setHover] = React.useState<boolean>(false);

  const handleClick = React.useCallback(() => {
    if (disabled === true) {
      return;
    }
    if (onClick) {
      onClick(status);
    }
  }, [status, disabled]);
  return (
    <svg width="190" height="190" viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg">
      <g
        transform="translate(-30, -25) "
        onMouseEnter={() => !disabled && setHover(true)}
        onMouseLeave={() => !disabled && setHover(false)}
        onClick={handleClick}
        cursor="pointer"
      >
        <g filter="url(#filter0_f_944_9033)">
          <circle cx={131} cy={131} r={20} fill="url(#paint0_radial_944_9033)" />
        </g>
        <path
          opacity={0.1}
          d="M131 195.5C166.622 195.5 195.5 166.622 195.5 131C195.5 95.3776 166.622 66.5 131 66.5C95.3776 66.5 66.5 95.3776 66.5 131C66.5 166.622 95.3776 195.5 131 195.5Z"
          stroke="white"
        />

        <circle opacity={0.6} cx={131} cy={131} r={68.5} stroke="white" />
        <g filter="url(#filter1_d_944_9033)">
          <circle cx={131} cy={131} r={64} fill="url(#paint1_radial_944_9033)" />
          <circle cx={131} cy={131} r={63} stroke="white" strokeWidth={2} />
        </g>
        <g opacity={0.5} filter="url(#filter2_f_944_9033)">
          <g clipPath="url(#clip0_944_9033)">
            <path
              d="M131 113C129.9 113 129 113.9 129 115V131C129 132.1 129.9 133 131 133C132.1 133 133 132.1 133 131V115C133 113.9 132.1 113 131 113ZM141.28 118.72C140.5 119.5 140.52 120.72 141.26 121.5C143.52 123.9 144.92 127.1 145 130.64C145.18 138.3 138.84 144.9 131.18 144.98C123.36 145.1 117 138.8 117 131C117 127.32 118.42 123.98 120.74 121.48C121.48 120.7 121.48 119.48 120.72 118.72C119.92 117.92 118.62 117.94 117.86 118.76C114.96 121.84 113.14 125.94 113 130.48C112.72 140.24 120.66 148.68 130.42 148.98C140.62 149.3 149 141.12 149 130.98C149 126.24 147.16 121.96 144.16 118.76C143.4 117.94 142.08 117.92 141.28 118.72Z"
              fill="white"
            />
          </g>
        </g>
        <g clipPath="url(#clip1_944_9033)">
          <path
            d="M131 113C129.9 113 129 113.9 129 115V131C129 132.1 129.9 133 131 133C132.1 133 133 132.1 133 131V115C133 113.9 132.1 113 131 113ZM141.28 118.72C140.5 119.5 140.52 120.72 141.26 121.5C143.52 123.9 144.92 127.1 145 130.64C145.18 138.3 138.84 144.9 131.18 144.98C123.36 145.1 117 138.8 117 131C117 127.32 118.42 123.98 120.74 121.48C121.48 120.7 121.48 119.48 120.72 118.72C119.92 117.92 118.62 117.94 117.86 118.76C114.96 121.84 113.14 125.94 113 130.48C112.72 140.24 120.66 148.68 130.42 148.98C140.62 149.3 149 141.12 149 130.98C149 126.24 147.16 121.96 144.16 118.76C143.4 117.94 142.08 117.92 141.28 118.72Z"
            fill="white"
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
          <radialGradient
            id="paint0_radial_944_9033"
            cx={0}
            cy={0}
            r={1}
            gradientUnits="userSpaceOnUse"
            gradientTransform="translate(131 131) rotate(90) scale(51)"
          >
            <stop stopColor="#1C1C1F" />
            <stop offset={1} stopColor="white" />
          </radialGradient>

          <clipPath id="clip0_944_9033">
            <rect width={48} height={48} fill="white" transform="translate(107 107)" />
          </clipPath>
          <clipPath id="clip1_944_9033">
            <rect width={48} height={48} fill="white" transform="translate(107 107)" />
          </clipPath>
        </defs>
      </g>
    </svg>
  );
};
