import { InfoOutlined } from '@mui/icons-material'
import { Tooltip, TooltipProps } from '@mui/material'
import React from 'react'

export const InfoTooltip = ({
  text,
  placement = 'bottom',
}: {
  text: string
  placement?: TooltipProps['placement']
}) => (
  <Tooltip title={text} arrow placement={placement}>
    <InfoOutlined fontSize="small" />
  </Tooltip>
)
