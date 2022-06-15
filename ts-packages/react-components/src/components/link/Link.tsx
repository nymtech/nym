import * as React from 'react';
import { Box, Typography, Link as MUILink, LinkProps as MUILinkProps } from '@mui/material';
import { OpenInNew } from '@mui/icons-material';

export interface LinkProps {
  text?: string;
  icon?: React.ReactNode;
  noIcon?: boolean;
}

export const Link = (props: MUILinkProps & LinkProps) => {
  const { text, icon, underline, noIcon, children } = props;
  let typoProps = {};
  if (!noIcon) {
    typoProps = { mr: 0.5 };
  }
  return (
    <MUILink
      {...props}
      sx={{
        display: 'inline-block',
        ':hover': {
          color: (theme) => theme.palette.nym.linkHover,
        },
      }}
      underline={underline || 'none'}
    >
      {children || (
        <Box
          sx={{
            display: 'flex',
            flexFlow: 'row nowrap',
            alignItems: 'center',
          }}
        >
          <Typography sx={{ ...typoProps, fontWeight: 400 }}>{text}</Typography>
          {!noIcon && (icon || <OpenInNew fontSize="inherit" />)}
        </Box>
      )}
    </MUILink>
  );
};
