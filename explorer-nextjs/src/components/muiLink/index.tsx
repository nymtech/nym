import { Link as MuiLink, type LinkProps as MuiLinkProps } from "@mui/material";
import NextLink, { type LinkProps as NextLinkProps } from "next/link";
// Imports
import type { ReactNode } from "react";

interface CustomLinkProps extends NextLinkProps, Omit<MuiLinkProps, "href"> {
  href: string;
  children: ReactNode;
}

// Component definition
export const Link = ({ href, children, ...props }: CustomLinkProps) => {
  return (
    <MuiLink
      component={NextLink}
      href={href}
      {...props}
      sx={{
        display: "flex",
        alignItems: "center",

        ...(props.sx || {}),
      }}
    >
      {children}
    </MuiLink>
  );
};
