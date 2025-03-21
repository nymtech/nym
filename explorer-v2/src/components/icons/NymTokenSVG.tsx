"use client";

import { useTheme } from "@mui/material/styles";
import * as React from "react";

export const NymTokenSVG = () => {
  const theme = useTheme();
  const color = theme.palette.text.primary;
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <title id="nymTokenTitle">Nym Token Icon</title>
      <g clipPath="url(#clip0_7134_15888)">
        <path
          d="M17.07 3.43C13.17 -0.480002 6.83 -0.480002 2.93 3.43C-0.980002 7.34 -0.980002 13.67 2.93 17.57C6.84 21.48 13.17 21.48 17.07 17.57C20.98 13.67 20.98 7.33 17.07 3.43ZM16.21 16.71C12.78 20.14 7.21 20.14 3.78 16.71C0.349997 13.28 0.349997 7.71 3.78 4.28C7.21 0.849997 12.78 0.849997 16.21 4.28C19.65 7.72 19.65 13.28 16.21 16.71Z"
          fill={color}
        />
        <path
          d="M15.4 16.33V4.66999C14.89 4.18999 14.32 3.76999 13.71 3.43999V14.59L6.35001 3.39999C5.71001 3.73999 5.12001 4.15999 4.60001 4.65999V16.33C5.11001 16.81 5.68001 17.23 6.29001 17.56V6.40999L13.65 17.6C14.29 17.26 14.88 16.83 15.4 16.33Z"
          fill={color}
        />
      </g>
      <defs>
        <clipPath id="clip0_7134_15888">
          <rect
            width="20"
            height="20"
            fill={color}
            transform="translate(0 0.5)"
          />
        </clipPath>
      </defs>
    </svg>
  );
};
