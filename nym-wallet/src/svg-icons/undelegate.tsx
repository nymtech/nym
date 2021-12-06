import React from 'react'
import { SvgIcon, SvgIconProps } from '@mui/material'

export const Undelegate = (props: SvgIconProps) => {
  return (
    <SvgIcon {...props}>
      <path d="M4 12V15H6V12H4ZM4 17H20V15H4V17Z" />
      <path d="M20 21C20 21.5523 19.5523 22 19 22H5C4.44772 22 4 21.5523 4 21V20H20V21Z" />
      <rect x="18" y="12" width="2" height="3" />
      <rect x="18" y="17" width="2" height="3" />
      <rect x="4" y="17" width="2" height="3" />
      <path d="M9.41 7.41L8 6L12 2L16 6L14.61 7.39L13 5.81L13 11L11 11L11 5.83L9.41 7.41Z" />
    </SvgIcon>
  )
}
