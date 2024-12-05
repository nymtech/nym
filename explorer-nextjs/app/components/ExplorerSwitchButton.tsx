import React, { useState } from "react";
import { Box, Button } from "@mui/material";

interface TwoSidedSwitchProps {
  leftLabel: string; // Label for the left side
  rightLabel: string; // Label for the right side
  onSwitch?: (side: "left" | "right") => void; // Callback when switched
}

const TwoSidedSwitch: React.FC<TwoSidedSwitchProps> = ({
  leftLabel,
  rightLabel,
  onSwitch,
}) => {
  const [selectedSide, setSelectedSide] = useState<"left" | "right">("left");

  const handleSwitch = (side: "left" | "right") => {
    setSelectedSide(side);
    if (onSwitch) onSwitch(side);
  };

  return (
    <Box
      sx={{
        display: "flex",
        borderRadius: "8px",
        overflow: "hidden",
        width: "200px",
        height: "40px",
        border: "2px solid black",
      }}
    >
      <Button
        onClick={() => handleSwitch("left")}
        sx={{
          flex: 1,
          backgroundColor: selectedSide === "left" ? "black" : "white",
          color: selectedSide === "left" ? "white" : "black",
          borderRadius: 0,
          "&:hover": {
            backgroundColor: selectedSide === "left" ? "black" : "lightgray",
          },
        }}
      >
        {leftLabel}
      </Button>
      <Button
        onClick={() => handleSwitch("right")}
        sx={{
          flex: 1,
          backgroundColor: selectedSide === "right" ? "black" : "white",
          color: selectedSide === "right" ? "white" : "black",
          borderRadius: 0,
          "&:hover": {
            backgroundColor: selectedSide === "right" ? "black" : "lightgray",
          },
        }}
      >
        {rightLabel}
      </Button>
    </Box>
  );
};

export default TwoSidedSwitch;
