import { InfoOutlined } from '@mui/icons-material'
import { Tooltip, TooltipProps } from '@mui/material'
import React from 'react'

export const InfoTooltip = ({
  title,
  placement = 'bottom',
}: {
  title: string
  placement?: TooltipProps['placement']
}) => (
  <Tooltip title={title} arrow placement={placement}>
    <InfoOutlined fontSize="small" />
  </Tooltip>
)
