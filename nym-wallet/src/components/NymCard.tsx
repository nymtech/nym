import React from 'react';
import { Box, Card, CardContent, CardHeader } from '@mui/material';
import { styled, Theme } from '@mui/material/styles';
import { Title } from './Title';

const CardContentNoPadding = styled(CardContent)(() => ({
  padding: 0,
  '&:last-child': {
    paddingBottom: 0,
  },
}));

export const NymCard: React.FC<{
  title: string | React.ReactElement;
  subheader?: string;
  Action?: React.ReactNode;
  Icon?: React.ReactNode;
  noPadding?: boolean;
  borderless?: boolean;
  dataTestid?: string;
}> = ({ title, subheader, Action, Icon, noPadding, borderless, children, dataTestid }) => (
  <Card variant="outlined" sx={{ overflow: 'auto', ...(borderless && { border: 'none', dropShadow: 'none' }) }}>
    <CardHeader
      sx={{ p: 3, color: (theme: Theme) => theme.palette.text.primary }}
      title={<Title title={title} Icon={Icon} />}
      subheader={subheader}
      data-testid={dataTestid || title}
      subheaderTypographyProps={{ variant: 'subtitle1' }}
      action={<Box sx={{ mt: 1, mr: 1 }}>{Action}</Box>}
    />
    {noPadding ? (
      <CardContentNoPadding>{children}</CardContentNoPadding>
    ) : (
      <CardContent sx={{ p: 3 }}>{children}</CardContent>
    )}
  </Card>
);
