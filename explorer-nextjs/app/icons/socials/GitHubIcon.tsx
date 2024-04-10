import * as React from 'react'
import { useTheme } from '@mui/material/styles'

interface GitHubIconProps {
  size?: number | string
  color?: string
}

export const GitHubIcon: FCWithChildren<GitHubIconProps> = ({
  size,
  color: colorProp,
}) => {
  const theme = useTheme()
  const color = colorProp || theme.palette.text.primary
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none">
      <g clipPath="url(#clip0_1223_2302)">
        <path
          fillRule="evenodd"
          clipRule="evenodd"
          d="M12.7 0C5.90002 0 0.400024 5.5 0.400024 12.3C0.400024 17.7 3.90002 22.3 8.80002 24C9.40002 24.1 9.60002 23.7 9.60002 23.4C9.60002 23.1 9.60002 22.1 9.60002 21.1C6.50002 21.7 5.70002 20.3 5.50002 19.7C5.40002 19.3 4.80002 18.3 4.20002 18C3.80002 17.8 3.20002 17.2 4.20002 17.2C5.20002 17.2 5.90002 18.1 6.10002 18.5C7.20002 20.4 9.00002 19.8 9.70002 19.5C9.80002 18.7 10.1 18.2 10.5 17.9C7.80002 17.6 4.90002 16.5 4.90002 11.8C4.90002 10.5 5.40002 9.4 6.20002 8.5C6.00002 8 5.60002 6.8 6.30002 5.1C6.30002 5.1 7.30002 4.8 9.70002 6.4C10.7 6.1 11.7 6 12.8 6C13.8 6 14.9 6.1 15.9 6.4C18.3 4.8 19.3 5.1 19.3 5.1C20 6.8 19.5 8.1 19.4 8.4C20.2 9.3 20.7 10.4 20.7 11.7C20.7 16.4 17.8 17.5 15.1 17.8C15.5 18.2 15.9 18.9 15.9 20.1C15.9 21.7 15.9 23.1 15.9 23.5C15.9 23.8 16.1 24.2 16.7 24.1C21.6 22.5 25.1 17.9 25.1 12.4C25 5.5 19.5 0 12.7 0Z"
          fill={color}
        />
      </g>
      <defs>
        <clipPath id="clip0_1223_2302">
          <rect width="24" height="24" transform="translate(0.400024)" />
        </clipPath>
      </defs>
    </svg>
  )
}
