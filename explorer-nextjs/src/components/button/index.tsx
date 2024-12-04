"use client";
import { colours } from "@/theme/colours";
import { Button as MUIButton } from "@mui/material";
import CircularProgress from "@mui/material/CircularProgress";
import Typography from "@mui/material/Typography";
import type { Theme } from "@mui/material/styles";
import Link from "next/link";
import type { ButtonProps } from "./types";

export const Button = ({
  text,
  handleClick,
  goToLink,
  isSuccess,
  isDisabled,
  isTextButton,
  isSmall,
  isXtraSmall,
  isFullWidth,
  isSecondary,
  isContrast,
  sx,
  buttonId,
  type,
  isLoading,
  startIcon,
  endIcon,
}: ButtonProps) => {
  const onClick = () => {
    if (goToLink) {
    } else if (handleClick) {
      handleClick();
    }
  };

  const fontColor = (t: Theme) => {
    if (isTextButton) return "primary.main";
    if (isSecondary)
      return isContrast ? "common.white" : "primary.contrastText";
    return t.palette.mode === "light"
      ? t.palette.common.white
      : t.palette.common.black;
  };
  const bgColor = (t: Theme) => {
    if (isTextButton || isSecondary) return "transparent";
    if (isDisabled)
      return t.palette.mode === "light" ? colours.pine[200] : colours.pine[800];
    return isSuccess ? "success.main" : "primary.main";
  };
  const hoverBgColor = () => {
    if (isTextButton || isSecondary) return "transparent";
    return isSuccess
      ? colours.green[500]
      : colours.pine[isDisabled ? 200 : 950];
  };

  const padding = isTextButton
    ? "none"
    : isXtraSmall
    ? "10px 23px"
    : isSmall
    ? "11px 23px"
    : "15px 23px";

  const CustomButtom = (
    <MUIButton
      type={type}
      sx={{
        textDecoration: "none",
        textAlign: "center",
        p: padding,
        borderRadius: 2,
        width: isFullWidth ? "100%" : "fit-content",
        height: isSmall ? 48 : 64,
        color: (t) => fontColor(t),
        letterSpacing: 0.45,
        backgroundColor: (t) => bgColor(t),
        border: isSecondary
          ? (t) =>
              isDisabled
                ? `1px solid ${t.palette.primary.main}`
                : `1px solid ${t.palette.primary.contrastText}`
          : "1px solid transparent",
        "&:hover": {
          backgroundColor: (t) => hoverBgColor(t),
        },
        "&:disabled": {
          color: "text.disabled",
        },
        ...sx,
      }}
      disabled={isDisabled}
      onClick={onClick}
      id={buttonId}
      startIcon={startIcon}
      endIcon={endIcon}
    >
      {!isLoading ? (
        <Typography
          variant={isSmall ? "body1" : isXtraSmall ? "body2" : "button"}
          sx={{
            textTransform: "none",
            fontWeight: "inherit",
            letterSpacing: 0,
          }}
        >
          {text}
        </Typography>
      ) : (
        <CircularProgress
          size={isSmall ? 20 : 24}
          sx={{ color: (t) => fontColor(t) }}
        />
      )}
    </MUIButton>
  );
  return goToLink ? (
    <Link href={goToLink.path} target={goToLink.target}>
      {CustomButtom}
    </Link>
  ) : (
    CustomButtom
  );
};
