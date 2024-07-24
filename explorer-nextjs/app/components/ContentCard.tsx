import React, { ReactEventHandler } from "react";
import { Card, CardHeader, CardContent, Typography } from "@mui/material";

type ContentCardProps = {
  title?: React.ReactNode;
  subtitle?: string;
  Icon?: React.ReactNode;
  Action?: React.ReactNode;
  errorMsg?: string;
  children: React.ReactNode;
  onClick?: ReactEventHandler;
};

export const ContentCard = ({
  title,
  Icon,
  Action,
  subtitle,
  errorMsg,
  children,
  onClick,
}: ContentCardProps) => (
  <Card onClick={onClick} sx={{ height: "100%" }}>
    {title && (
      <CardHeader
        title={title || ""}
        avatar={Icon}
        action={Action}
        subheader={subtitle}
      />
    )}
    {children && <CardContent>{children}</CardContent>}
    {errorMsg && (
      <Typography variant="body2" sx={{ color: "danger", padding: 2 }}>
        {errorMsg}
      </Typography>
    )}
  </Card>
);
