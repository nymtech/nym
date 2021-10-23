import { Card, CardHeader, CardContent, Typography } from '@mui/material';
import React, { ReactEventHandler } from 'react';
import { MainContext } from 'src/context/main';

type ContentCardProps = {
  title?: string | React.ReactNode;
  subtitle?: string;
  Icon?: React.ReactNode;
  Action?: React.ReactNode;
  errorMsg?: string;
  onClick?: ReactEventHandler;
};

export const ContentCard: React.FC<ContentCardProps> = ({
  title,
  Icon,
  Action,
  subtitle,
  errorMsg,
  children,
  onClick,
}) => {
  const { mode } = React.useContext(MainContext);
  return (
    <Card
      onClick={onClick}
      sx={{
        background: (theme) =>
          mode === 'dark'
            ? theme.palette.secondary.dark
            : theme.palette.primary.light,
      }}
    >
      <CardHeader
        sx={{
          color: (theme) =>
            mode === 'dark' ? theme.palette.primary.main : 'secondary.main',
        }}
        title={title || ''}
        avatar={Icon}
        action={Action}
        subheader={subtitle}
      />
      {children && <CardContent>{children}</CardContent>}
      {errorMsg && (
        <Typography variant="body2" sx={{ color: 'danger', padding: 2 }}>
          {errorMsg}
        </Typography>
      )}
    </Card>
  );
};

ContentCard.defaultProps = {
  title: undefined,
  subtitle: undefined,
  Icon: null,
  Action: null,
  errorMsg: undefined,
  onClick: () => null,
};
