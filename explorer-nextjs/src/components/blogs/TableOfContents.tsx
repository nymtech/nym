import { Card, CardContent, CardHeader, Typography } from "@mui/material";
import { Link } from "../muiLink";

const TableOfContents = ({
  headings,
}: {
  headings: { id: string; heading: string }[];
}) => {
  return (
    <Card
      elevation={0}
      sx={{
        width: "100%",
        display: {
          xs: "none",
          md: "block",
        },
        p: 4,
        position: "sticky",
        top: 50,
      }}
    >
      <CardHeader title="Table of contents" />
      <CardContent>
        {headings.map((heading) => (
          <Link href={`#${heading.id}`} key={heading.id}>
            <Typography variant="body2" sx={{ mb: 3 }}>
              {heading.heading}
            </Typography>
          </Link>
        ))}
      </CardContent>
    </Card>
  );
};

export default TableOfContents;
