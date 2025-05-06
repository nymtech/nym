"use client";
import { Button, ButtonGroup } from "@mui/material";

type Option = {
  label: string;
  isSelected: boolean;
  value: "all" | "mixnodes" | "gateways";
};

type Options = [Option, Option, Option];

const NodeFilterButtonGroup = ({
  size = "small",
  options,
  onPage,
  onFilterChange,
}: {
  size?: "small" | "medium" | "large";
  options: Options;
  onPage: string;
  onFilterChange: (filter: "all" | "mixnodes" | "gateways") => void;
}) => {
  const handleClick = (value: "all" | "mixnodes" | "gateways") => {
    if (onPage === value) return;
    onFilterChange(value);
  };
  return (
    <ButtonGroup size={size}>
      {options.map((option) => (
        <Button
          key={option.label}
          onClick={() => handleClick(option.value)}
          sx={{
            color: option.isSelected ? "primary.contrastText" : "text.primary",
            "&:hover": {
              bgcolor: option.isSelected ? "primary.main" : "",
            },
            bgcolor: option.isSelected ? "primary.main" : "transparent",
          }}
          variant="outlined"
        >
          {option.label}
        </Button>
      ))}
    </ButtonGroup>
  );
};

export default NodeFilterButtonGroup;
