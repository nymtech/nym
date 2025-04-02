import Button, {
  type ButtonProps as MUIButtonProps,
} from "@mui/material/Button";
import Link from "next/link";
import type React from "react";

interface ButtonOrLinkProps extends Omit<MUIButtonProps, "variant"> {
  href?: string;
  variant?: "outlined" | "contained" | "text";
  isSelected?: boolean;
}

export const ButtonOrLink: React.FC<ButtonOrLinkProps> = ({
  href,
  variant = "contained",
  isSelected,
  children,
  ...props
}) => {
  const selectedStyle = {
    contained: {
      outline: "1px solid",
      outlineColor: "primary.main",
      borderColor: "primary.main",
    },
    outlined: {
      borderStyle: "solid",
      backgroundColor: "primary.main",
      color: "background.main",
      "&:hover": {
        backgroundColor: "primary.main",
      },
    },
    text: {
      color: "primary.main",
    },
  };
  return href ? (
    <Button
      sx={{
        ...(isSelected && selectedStyle[variant]),
        textAlign: "center",
      }}
      component={Link}
      href={href}
      variant={variant}
      {...props}
    >
      {children}
    </Button>
  ) : (
    <Button
      sx={{
        ...(isSelected && selectedStyle[variant]),
        textAlign: "center",
      }}
      variant={variant}
      {...props}
    >
      {children}
    </Button>
  );
};
