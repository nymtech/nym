import ArrowUpRight from "@/components/icons/ArrowUpRight";
import {
  Card,
  CardContent,
  CardHeader,
  Stack,
  type SxProps,
  Typography,
} from "@mui/material";
import { Link } from "../muiLink";

const cardStyles = {
  p: 3,
  cursor: "pointer",
  "&:hover": {
    bgcolor: "accent.main",
  },
};
const cardContentStyles = {
  mt: 10,
};
const titleStyles = {
  letterSpacing: 0.7,
};

const ExplorerHeroCard = ({
  title,
  label,
  description,
  image,
  link,
  sx,
}: {
  title: string;
  label: string;
  description: string;
  image: React.ReactNode;
  link: string;
  sx?: SxProps;
}) => {
  return (
    <Link href={link} sx={{ textDecoration: "none" }}>
      <Card sx={{ ...cardStyles, ...sx }} elevation={0}>
        <CardHeader
          title={
            <Stack direction="row" justifyContent="space-between">
              <Typography variant="body4" sx={titleStyles}>
                {label}
              </Typography>
              <ArrowUpRight />
            </Stack>
          }
        />
        <CardContent sx={cardContentStyles}>
          <Stack spacing={4}>
            {image}
            <Typography variant="h2">{title}</Typography>
            <Typography variant="body3">{description}</Typography>
          </Stack>
        </CardContent>
      </Card>
    </Link>
  );
};
export default ExplorerHeroCard;