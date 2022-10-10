import React from 'react';
import { Card, CardContent, CardHeader } from '@mui/material';
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
      sx={{
        p: 3,
        color: 'text.primary',
        '& .MuiCardHeader-title h5': { fontSize: '1.25rem' },
      }}
      title={<Title title={title} Icon={Icon} />}
      subheader={subheader}
      data-testid={dataTestid || title}
      subheaderTypographyProps={{ variant: 'subtitle1' }}
      action={Action}
    />
    {noPadding ? (
      <CardContentNoPadding>{children}</CardContentNoPadding>
    ) : (
      <CardContent sx={{ p: 3, paddingTop: 0 }}>{children}</CardContent>
    )}
  </Card>
);
