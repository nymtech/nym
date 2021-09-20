import { Card, CardHeader, CardContent } from '@mui/material';
import React from 'react';

export const ContentCard: React.FC<{
  title: string;
  subtitle?: string;
  Icon?: React.ReactNode;
  Action?: React.ReactNode;
}> = ({ title, Icon, Action, subtitle, children }) => (
  <Card sx={{ m: 3 }}>
    <CardHeader
      title={title}
      avatar={Icon}
      action={Action}
      subheader={subtitle}
    />
    {children && <CardContent>{children}</CardContent>}
  </Card>
);
