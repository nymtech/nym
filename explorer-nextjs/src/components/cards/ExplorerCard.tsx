import { Card, CardContent, CardHeader, type SxProps } from "@mui/material";

const cardStyles = {
  p: 2,
};

const cardTitleStyles: SxProps = {
  variant: "h5",
  color: "pine.600",
  letterSpacing: 0.7,
};
const cardSubtitleStyles: SxProps = {
  variant: "h2",
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
      />
      <CardContent>{children}</CardContent>
    </Card>
  );
};

export default ExplorerCard;
