import { Card, CardContent, CardHeader, type SxProps } from "@mui/material";

const cardStyles = {
  p: 3,
  height: "100%",
  display: "flex",
  flexDirection: "column",
  justifyContent: "space-between",
  alignItems: "stretch",
  flexGrow: 1,
};

const cardTitleStyles: SxProps = {
  variant: "h5",
  color: "pine.600",
  letterSpacing: 0.7,
};
const cardSubtitleStyles: SxProps = {
  variant: "h3",
  mt: 3,
  color: "pine.400",
};

const ExplorerCard = ({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}) => {
  return (
    <Card elevation={0} sx={cardStyles}>
      <CardHeader
        title={title}
        titleTypographyProps={cardTitleStyles}
        subheader={subtitle}
        subheaderTypographyProps={cardSubtitleStyles}
        sx={{ padding: 0 }}
      />
      <CardContent sx={{ padding: 0 }}>{children}</CardContent>
    </Card>
  );
};

export default ExplorerCard;
