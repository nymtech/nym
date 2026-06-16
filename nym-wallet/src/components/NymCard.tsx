import React from 'react';
import { Card, CardContent, CardHeader, SxProps } from '@mui/material';
import { styled, Theme } from '@mui/material/styles';
import { Title } from './Title';

const CardContentNoPadding = styled(CardContent)(() => ({
  padding: 0,
  '&:last-child': {
    paddingBottom: 0,
  },
}));

export const NymCard: FCWithChildren<{
  title?: string | React.ReactElement;
  subheader?: React.ReactNode;
  Action?: React.ReactNode;
  Icon?: React.ReactNode;
  noPadding?: boolean;
  borderless?: boolean;
  /** Omit the card header row (use for fully custom headers inside children). */
  hideHeader?: boolean;
  /** Preferred: standard DOM test id. `dataTestid` (camelCase) is kept for back-compat. */
  'data-testid'?: string;
  dataTestid?: string;
  sx?: SxProps;
  sxTitle?: SxProps;
  children?: React.ReactNode;
}> = ({
  title,
  subheader,
  Action,
  Icon,
  noPadding,
  borderless,
  hideHeader,
  children,
  dataTestid,
  'data-testid': dataTestidAttr,
  sx,
  sxTitle,
}) => (
  <Card
    variant="outlined"
    // Resolved id on the root element (wraps content, so it can scope children).
    data-testid={dataTestidAttr ?? dataTestid}
    sx={{
      overflow: 'hidden',
      borderRadius: 4,
      borderColor: 'divider',
      backgroundImage: 'none',
      ...(borderless && { border: 'none', boxShadow: 'none' }),
      ...sx,
    }}
  >
    {!hideHeader && title !== undefined && (
      <CardHeader
        sx={{
          p: 3,
          color: (theme: Theme) => theme.palette.text.primary,
          '& .MuiCardHeader-title h5': { fontSize: '1.25rem' },
          '& .MuiCardHeader-action': {
            alignSelf: 'center',
            m: 0,
          },
        }}
        title={<Title title={title} Icon={Icon} sx={sxTitle} />}
        subheader={subheader}
        subheaderTypographyProps={{ variant: 'subtitle1' }}
        action={Action}
      />
    )}
    {noPadding ? (
      <CardContentNoPadding>{children}</CardContentNoPadding>
    ) : (
      <CardContent sx={{ p: 3, paddingTop: hideHeader ? 3 : 0 }}>{children}</CardContent>
    )}
  </Card>
);
