import React from 'react';
import { Link as MuiLink } from '@mui/material';
import { Link as RRDL } from 'react-router-dom';

type StyledLinkProps = {
  to: string;
  children: string;
  target?: React.HTMLAttributeAnchorTarget;
  dataTestId?: string;
  color?: string;
};

const StyledLink = ({ to, children, dataTestId, target, color = 'inherit' }: StyledLinkProps) => (
  <MuiLink color={color} target={target} underline="none" component={RRDL} to={to} data-testid={dataTestId}>
    {children}
  </MuiLink>
);

export default StyledLink;
