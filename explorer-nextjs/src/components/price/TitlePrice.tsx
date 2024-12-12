import { Box, Typography } from "@mui/material";
import type React from "react";
import { NymTokenSVG } from "../icons/NymTokenSVG";
import {
  type IUpDownPriceIndicatorProps,
  UpDownPriceIndicator,
} from "./UpDownPriceIndicator";

interface ITitlePriceProps {
  price: number;
  upDownLine: IUpDownPriceIndicatorProps;
}
export const TitlePrice = (props: ITitlePriceProps): React.ReactNode => {
  const { price, upDownLine } = props;
  return (
    <Box display={"flex"} flexDirection={"column"} alignItems={"flex-end"}>
      <Box display={"flex"} justifyContent={"space-between"} width={"100%"}>
        <Box display={"flex"} gap={1} alignItems={"center"}>
          <NymTokenSVG />
          <Typography variant="h3" sx={{ color: "pine.400" }}>
            NYM
          </Typography>
        </Box>
        <Typography variant="h3" sx={{ color: "pine.400" }}>
          ${price}
        </Typography>
      </Box>
      <UpDownPriceIndicator {...upDownLine} />
    </Box>
  );
};
