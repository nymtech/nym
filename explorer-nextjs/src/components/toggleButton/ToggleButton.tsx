"use client";
import { Button, ButtonGroup, CircularProgress } from "@mui/material";
import { useState } from "react";
import { Link } from "../muiLink";

type Option = {
  label: string;
  isSelected: boolean;
  link: string;
};

type Options = [Option, Option];

const ExplorerButtonGroup = ({
  size = "small",
  options,
  onPage,
}: {
  size?: "small" | "medium" | "large";
  options: Options;
  onPage: string;
}) => {
  const [loading, setLoading] = useState<string | null>(null);
  const handleClick = (label: string) => {
    if (onPage === label) return;
    setLoading(label);
  };
  return (
    <ButtonGroup size={size}>
      {options.map((option) => (
        <Link
          href={option.link}
          key={option.label}
          sx={{ textDecoration: "none" }}
          onClick={() => handleClick(option.label)}
        >
          <Button
            sx={{
              color: option.isSelected
                ? "primary.contrastText"
                : "text.primary",
              "&:hover": {
                bgcolor: option.isSelected ? "primary.main" : "",
              },
              bgcolor: option.isSelected ? "primary.main" : "transparent",
            }}
            variant="outlined"
          >
            {loading === option.label ? (
              <CircularProgress size={18} color="inherit" />
            ) : (
              option.label
            )}
          </Button>
        </Link>
      ))}
    </ButtonGroup>
  );
};
export default ExplorerButtonGroup;
