import Link from "@mui/material/Link";
import NextLink from "next/link";

export function SiteLogo() {
  return (
    <Link
      component={NextLink}
      underline="hover"
      sx={{ color: "text.primary", fontSize: "18px" }}
      href="/"
    >
      Nym Node Status
    </Link>
  );
}
