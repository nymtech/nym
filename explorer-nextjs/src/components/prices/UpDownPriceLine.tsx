import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import { Box, Typography } from "@mui/material";
import type { ReactElement } from "react";

interface ICardUpDownPriceLineProps {
  percentage: number;
  numberWentUp: boolean;
}
export const CardUpDownPriceLine = (
  props: ICardUpDownPriceLineProps,
): ReactElement => {
  const { percentage, numberWentUp } = props;
  return (
    <Box display={"flex"} alignItems={"center"}>
      {numberWentUp ? (
        <ArrowUpwardIcon sx={{ color: "#00CA33", fontSize: 13 }} />
      ) : (
        <ArrowDownwardIcon sx={{ color: "#DF1400", fontSize: 13 }} />
      )}
      <Typography
        fontSize={13}
        sx={{ color: numberWentUp ? "#00CA33" : "#DF1400" }}
      >
        {percentage}% (24H)
      </Typography>
    </Box>
  );
};
