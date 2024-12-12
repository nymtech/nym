import { Box, Button } from "@mui/material";
import type React from "react";
import { useState } from "react";

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
        borderRadius: "20px",
        overflow: "hidden",
        width: "200px",
        height: "40px",
      }}
    >
      <Button
        onClick={() => handleSwitch("left")}
        sx={{
          flex: 1,
          backgroundColor: selectedSide === "left" ? "black" : "transparent",
          color: selectedSide === "left" ? "white" : "black",
          border: "1px dashed black",
          borderRight: "none",
          borderBottomRightRadius: 0,
          borderTopRightRadius: 0,
          //   "&:hover": {
          //     backgroundColor: selectedSide === "left" ? "black" : "lightgray",
          //   },
        }}
      >
        {leftLabel}
      </Button>
      <Button
        onClick={() => handleSwitch("right")}
        sx={{
          flex: 1,
          backgroundColor: selectedSide === "right" ? "black" : "transparent",
          color: selectedSide === "right" ? "white" : "black",
          border: "1px dashed black",
          borderLeft: "none",
          borderBottomLeftRadius: 0,
          borderTopLeftRadius: 0,
          //   "&:hover": {
          //     backgroundColor: selectedSide === "right" ? "black" : "lightgray",
          //   },
        }}
      >
        {rightLabel}
      </Button>
    </Box>
  );
};

export default TwoSidedSwitch;
