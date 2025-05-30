// Types
import type { components } from "../../app/lib/strapi";

// Components
import { Link } from "./MuiLink";

// MUI Components
import { Box, Typography } from "@mui/material";
import Grid2 from "@mui/material/Grid2";

export const FooterLinks = ({
  linkBlocks = [],
}: {
  linkBlocks: components["schemas"]["FooterLinkBlockComponent"][];
}) => {
  return (
    <Grid2
      container
      spacing={{ xs: 2, md: 3 }}
      columns={{ xs: 1, sm: 8, md: 5 }}
    >
      {linkBlocks?.map((block) => {
        return (
          <Grid2 key={block.id} size={{ xs: 1, sm: 4, md: 1 }}>
            <Typography
              component={block?.heading?.level || "h3"}
              variant="subtitle1"
              sx={{ mb: 4 }}
            >
              {block?.heading?.title}
            </Typography>
            <Box
              component={"ul"}
              sx={{
                display: "flex",
                flexDirection: "column",
                gap: 2,
              }}
            >
              {block?.links?.map((link) => {
                const isLinkExternal = link.url?.startsWith("http");
                return (
                  <Box
                    sx={{
                      listStyle: "none",
                    }}
                    component={"li"}
                    key={link.id}
                  >
                    <Link
                      href={
                        isLinkExternal
                          ? (link.url ?? "/")
                          : link.url
                            ? `https://nym.com${link.url}`
                            : "/"
                      }
                      target="_blank"
                      rel="noopener noreferrer"
                      sx={{
                        textDecoration: "none",
                        "&:hover": {
                          textDecoration: "underline",
                        },
                      }}
                    >
                      <Typography variant="body3">
                        {link.title}
                        {isLinkExternal ? " ↗" : ""}
                      </Typography>
                    </Link>
                  </Box>
                );
              })}
            </Box>
          </Grid2>
        );
      })}
    </Grid2>
  );
};
