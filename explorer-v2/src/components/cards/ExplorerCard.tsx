import {
  Card,
  CardContent,
  CardHeader,
  useTheme,
  type SxProps,
} from "@mui/material";

const cardStyles = {
  p: 2,
  bgcolor: "background.paper",
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
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  const cardLabelStyles: SxProps = {
    variant: "h5",
    color: isDarkMode ? "pine.300" : "pine.600",
    letterSpacing: 0.7,
  };

  const cardTitleStyles: SxProps = {
    variant: "h2",
    mt: 3,
    color: isDarkMode ? "base.white" : "pine.950",
  };

  return (
    <Card elevation={0} sx={{ ...cardStyles, ...sx }}>
      <CardHeader
        title={label}
        subheader={title}
        slotProps={{ subheader: cardTitleStyles, title: cardLabelStyles }}
      />
      <CardContent>{children}</CardContent>
    </Card>
  );
};

export default ExplorerCard;
