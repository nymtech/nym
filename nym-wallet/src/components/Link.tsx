import React from 'react';
import { Box, Typography, Link as MUILink, LinkProps as MUILinkProps, TypographyProps } from '@mui/material';
import { OpenInNew } from '@mui/icons-material';

export interface LinkProps {
  text: string;
  icon?: React.ReactNode;
}

export const Link = (props: MUILinkProps & TypographyProps & LinkProps) => {
  const { text, icon, underline } = props;
  return (
    <MUILink
      {...props}
      sx={{
        display: 'inline-block',
        ':hover': {
          color: (theme) => theme.palette.nym.nymWallet.text.linkHover,
        },
      }}
      underline={underline || 'none'}
    >
      <Box
        sx={{
          display: 'flex',
          flexFlow: 'row nowrap',
          alignItems: 'end',
        }}
      >
        <Typography sx={{ mr: 0.5, fontWeight: 400 }}>{text}</Typography>
        {icon || <OpenInNew fontSize="inherit" />}
      </Box>
    </MUILink>
  );
};
