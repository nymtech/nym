// Types
import type { components } from "@/types/strapi";

// Components
import { Link } from "@/components/muiLink";

// MUI Components
import { Box, Grid2, Typography } from "@mui/material";

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
                      href={link?.url || ""}
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
