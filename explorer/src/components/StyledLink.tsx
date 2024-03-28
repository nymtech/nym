import React from 'react';
import { Link as MuiLink, SxProps } from '@mui/material';
import { Link as RRDL } from 'react-router-dom';

type StyledLinkProps = {
  to: string;
  children: string;
  target?: React.HTMLAttributeAnchorTarget;
  dataTestId?: string;
  color?: string;
  sx?: SxProps;
};

const StyledLink = ({ to, children, dataTestId, target, color = 'inherit', sx }: StyledLinkProps) => (
  <MuiLink
    sx={{ ...sx }}
    color={color}
    target={target}
    underline="none"
    component={RRDL}
    to={to}
    data-testid={dataTestId}
  >
    {children}
  </MuiLink>
);

export default StyledLink;
