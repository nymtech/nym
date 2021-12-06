import React from 'react'
import { SvgIcon, SvgIconProps } from '@mui/material'

export const Delegate = (props: SvgIconProps) => {
  return (
    <SvgIcon {...props}>
      <path d="M4 12V15H6V12H4ZM16 7L14.59 5.59L13 7.17V2H11V7.19L9.39 5.61L8 7L12 11L16 7ZM4 17H20V15H4V17Z" />
      <path d="M20 21C20 21.5523 19.5523 22 19 22H5C4.44772 22 4 21.5523 4 21V20H20V21Z" />
      <rect x="18" y="12" width="2" height="3" />
      <rect x="18" y="17" width="2" height="3" />
      <rect x="4" y="17" width="2" height="3" />
    </SvgIcon>
  )
}
