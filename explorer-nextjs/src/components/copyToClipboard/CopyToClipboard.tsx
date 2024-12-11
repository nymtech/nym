"use client";

import { IconButton } from "@mui/material";
import { CopyToClipboard as ReactCopyToClipboard } from "react-copy-to-clipboard";
import CopyFile from "../icons/CopyFile";

const CopyToClipboard = ({
  text,
  Icon,
  size = "medium",
}: {
  text: string;
  Icon?: React.ReactNode;
  size?: "small" | "medium" | "large";
}) => {
  return (
    <ReactCopyToClipboard text={text}>
      <IconButton size={size}>{Icon || <CopyFile />}</IconButton>
    </ReactCopyToClipboard>
  );
};

export default CopyToClipboard;
