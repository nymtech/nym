"use client";

import { IconButton, Typography, useTheme } from "@mui/material";
import { useCopyToClipboard } from "@uidotdev/usehooks";
import { useEffect } from "react";
import CopyFile from "../icons/CopyFile";
import CopyFileDark from "../icons/CopyFileDark";

const CLEAR_AFTER_MS = 10_000;

const CopyToClipboard = ({
  text,
  Icon,
  size = "medium",
}: {
  text: string;
  Icon?: React.ReactNode;
  size?: "small" | "medium" | "large";
}) => {
  const [copiedText, setCopiedText] = useCopyToClipboard();
  const hasCopied = Boolean(copiedText);
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  useEffect(() => {
    if (hasCopied) {
      const timeout = setTimeout(() => {
        setCopiedText("");
      }, CLEAR_AFTER_MS);

      return () => clearTimeout(timeout);
    }
  }, [hasCopied, setCopiedText]);

  if (hasCopied) {
    return (
      <Typography
        sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
        variant="h6"
        color="textSecondary"
      >
        Copied
      </Typography>
    );
  }

  return (
    <IconButton size={size} onClick={() => setCopiedText(text)}>
      {Icon || (isDarkMode ? <CopyFileDark /> : <CopyFile />)}
    </IconButton>
  );
};

export default CopyToClipboard;
