import { colours } from "@/theme/colours";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import { Box, Typography } from "@mui/material";
import type React from "react";
import type { ReactElement } from "react";

export interface IUpDownPriceIndicatorProps {
  percentage: number;
  numberWentUp: boolean;
}
export const UpDownPriceIndicator = (
  props: IUpDownPriceIndicatorProps,
): ReactElement => {
  const { percentage, numberWentUp } = props;
  return (
    <Box display={"flex"} alignItems={"center"}>
      {numberWentUp ? (
        <ArrowUpwardIcon sx={{ color: colours.alert.success, fontSize: 12 }} />
      ) : (
        <ArrowDownwardIcon sx={{ color: colours.alert.error, fontSize: 12 }} />
      )}
      <Typography
        variant="subtitle3"
        sx={{
          color: numberWentUp ? colours.alert.success : colours.alert.error,
        }}
      >
        {percentage}% (24H)
      </Typography>
    </Box>
  );
};
