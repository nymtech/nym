"use client";
import { Button, ButtonGroup, Stack } from "@mui/material";

type Option = {
  label: string;
  isSelected: boolean;
  value: "all" | "mixnodes" | "gateways" | "recommended";
};

type Options = [Option, Option, Option, Option];

const NodeFilterButtonGroup = ({
  size = "small",
  options,
  onPage,
  onFilterChange,
}: {
  size?: "small" | "medium" | "large";
  options: Options;
  onPage: string;
  onFilterChange: (
    filter: "all" | "mixnodes" | "gateways" | "recommended"
  ) => void;
}) => {
  const handleClick = (
    value: "all" | "mixnodes" | "gateways" | "recommended"
  ) => {
    if (onPage === value) return;
    onFilterChange(value);
  };

  const getMobileButtonStyles = (isSelected: boolean) => ({
    color: isSelected ? "primary.contrastText" : "text.primary",
    "&:hover": {
      bgcolor: isSelected ? "primary.main" : "",
    },
    bgcolor: isSelected ? "primary.main" : "transparent",
    width: "100%",
    py: 1.5,
    px: 2,
  });

  const getDesktopButtonStyles = (isSelected: boolean) => ({
    color: isSelected ? "primary.contrastText" : "text.primary",
    "&:hover": {
      bgcolor: isSelected ? "primary.main" : "",
    },
    bgcolor: isSelected ? "primary.main" : "transparent",
  });

  return (
    <>
      {/* Mobile view - Stack */}
      <Stack
        spacing={1.5}
        sx={{
          display: { xs: "flex", sm: "none" },
          width: "100%",
        }}
      >
        {options.map((option) => (
          <Button
            key={option.label}
            onClick={() => handleClick(option.value)}
            sx={getMobileButtonStyles(option.isSelected)}
            variant="outlined"
          >
            {option.label}
          </Button>
        ))}
      </Stack>

      {/* Desktop view - ButtonGroup */}
      <ButtonGroup size={size} sx={{ display: { xs: "none", sm: "flex" } }}>
        {options.map((option) => (
          <Button
            key={option.label}
            onClick={() => handleClick(option.value)}
            sx={getDesktopButtonStyles(option.isSelected)}
            variant="outlined"
          >
            {option.label}
          </Button>
        ))}
      </ButtonGroup>
    </>
  );
};

export default NodeFilterButtonGroup;
