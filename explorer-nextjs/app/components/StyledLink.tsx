import React from 'react'
import { Link as MuiLink, SxProps, Typography } from '@mui/material'
import Link from 'next/link'

type StyledLinkProps = {
  to: string
  children: string
  target?: React.HTMLAttributeAnchorTarget
  dataTestId?: string
  color?: string
  sx?: SxProps
}

const StyledLink = ({
  to,
  children,
  dataTestId,
  target,
  color,
  sx,
}: StyledLinkProps) => (
  <Link
    href={to}
    target={target}
    data-testid={dataTestId}
    style={{ textDecoration: 'none' }}
  >
    <Typography component="a" sx={{ ...sx }} color={color}>
      {children}
    </Typography>
  </Link>
)

export default StyledLink
