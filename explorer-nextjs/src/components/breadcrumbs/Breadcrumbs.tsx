import BreadcrumbsMUI from "@mui/material/Breadcrumbs";
import Typography from "@mui/material/Typography";
import Link from "next/link";

interface BreadcrumbComponentProps {
  items: BreadcrumbItemType[];
}

interface BreadcrumbItemType {
  label: string;
  href?: string;
  isCurrentPage?: boolean;
}

export const Breadcrumbs = ({ items }: BreadcrumbComponentProps) => {
  return (
    <BreadcrumbsMUI
      sx={{
        color: "primary.main",
        display: "flex",
        margin: "0",
        padding: "0",
        alignItems: "center",
      }}
      separator={
        <Typography
          sx={{ display: "flex", height: "100%" }}
          variant="subtitle3"
        >
          /
        </Typography>
      }
      aria-label="breadcrumb"
    >
      {items.map((item) => {
        // Check if it's the current page
        if (item.isCurrentPage) {
          return (
            <Typography
              sx={{
                display: "flex",
              }}
              variant="subtitle3"
              key={item.label}
            >
              {item.label}
            </Typography>
          );
        }

        // If it's not the current page, render a clickable link
        return (
          <Link key={item.label} href={item.href || "#"} passHref>
            <Typography
              sx={{
                display: "flex",
                "&:hover": {
                  textDecoration: "underline",
                },
              }}
              variant="subtitle3"
            >
              {item.label}
            </Typography>
          </Link>
        );
      })}
    </BreadcrumbsMUI>
  );
};
