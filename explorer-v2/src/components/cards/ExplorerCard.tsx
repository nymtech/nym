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
  label,
  title,
  children,
  sx,
}: {
  label: string | React.ReactNode;
  title?: string;
  children: React.ReactNode;
  sx?: SxProps;
}) => {
  return (
    <Card elevation={0} sx={{ ...cardStyles, ...sx }}>
      <CardHeader
        title={label}
        titleTypographyProps={cardTitleStyles}
        subheader={title}
        subheaderTypographyProps={cardSubtitleStyles}
      />
      <CardContent>{children}</CardContent>
    </Card>
  );
};

export default ExplorerCard;
