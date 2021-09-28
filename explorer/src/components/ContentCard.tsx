import { Card, CardHeader, CardContent, Typography } from '@mui/material';
import React from 'react';

export const ContentCard: React.FC<{
  title: string;
  subtitle?: string;
  Icon?: React.ReactNode;
  Action?: React.ReactNode;
  errorMsg?: string;
}> = ({ title, Icon, Action, subtitle, errorMsg, children }) => (
  <Card sx={{ m: 3 }}>
    <CardHeader
      title={title}
      avatar={Icon}
      action={Action}
      subheader={subtitle}
    />
    {children && <CardContent>{children}</CardContent>}
    {errorMsg && (
      <Typography
        variant="body2"
        sx={{ color: 'danger', padding: 2 }}
      >
        {errorMsg}
      </Typography>
    )}
  </Card>
);
