import { Card, CardHeader, CardContent, Typography, Box } from "@mui/material";
import React, { FC, ReactElement, ReactEventHandler } from "react";
import { ExplorerLineChart, IExplorerLineChartData } from "./ExplorerLineChart";
import {
  ExplorerProgressBar,
  IExplorerProgressBarProps,
} from "./ExplorerProgressBar";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import { NymTokenSVG } from "../icons/NymTokenSVG";

interface ICardUpDownPriceLineProps {
  percentage: number;
  numberWentUp: boolean;
}
const CardUpDownPriceLine = (
  props: ICardUpDownPriceLineProps
): ReactElement => {
  const { percentage, numberWentUp } = props;
  return (
    <Box mb={3} display={"flex"}>
      {numberWentUp ? (
        <ArrowUpwardIcon sx={{ color: "#00CA33" }} fontSize="small" />
      ) : (
        <ArrowDownwardIcon sx={{ color: "#DF1400" }} fontSize="small" />
      )}
      <Typography sx={{ color: numberWentUp ? "#00CA33" : "#DF1400" }}>
        {percentage}% (24H)
      </Typography>
    </Box>
  );
};

interface ICardTitlePriceProps {
  price: number;
  upDownLine: ICardUpDownPriceLineProps;
}
const CardTitlePrice = (props: ICardTitlePriceProps): React.ReactNode => {
  const { price, upDownLine } = props;
  return (
    <Box display={"flex"} flexDirection={"column"} alignItems={"flex-end"}>
      <Box display={"flex"} justifyContent={"space-between"} width={"100%"}>
        <Box display={"flex"} gap={1}>
          <NymTokenSVG />
          <Typography>NYM</Typography>
        </Box>
        <Typography>${price}</Typography>
      </Box>
      <CardUpDownPriceLine {...upDownLine} />
    </Box>
  );
};

interface ICardDataRowsProps {
  rows: Array<{ key: string; value: string }>;
}
const CardDataRows = (props: ICardDataRowsProps): React.ReactNode => {
  const { rows } = props;
  return (
    <Box mb={3}>
      {rows.map((row, i) => {
        return (
          <Box
            key={i}
            paddingTop={2}
            paddingBottom={2}
            display={"flex"}
            justifyContent={"space-between"}
            borderBottom={i === 0 ? "1px solid #fff" : "none"}
          >
            <Typography>{row.key}</Typography>
            <Typography>{row.value}</Typography>
          </Box>
        );
      })}
    </Box>
  );
};

type ContentCardProps = {
  overTitle?: string;
  title?: string | number;
  upDownLine?: ICardUpDownPriceLineProps;
  titlePrice?: ICardTitlePriceProps;
  dataRows?: ICardDataRowsProps;
  graph?: Array<IExplorerLineChartData>;
  progressBar?: IExplorerProgressBarProps;
  paragraph?: string;
  onClick?: ReactEventHandler;
};

export const ExplorerCard: FC<ContentCardProps> = ({
  title,
  titlePrice,
  overTitle,
  upDownLine,
  dataRows,
  graph,
  progressBar,
  paragraph,
  onClick,
}) => (
  <Card onClick={onClick} sx={{ height: "100%" }}>
    <CardContent>
      {overTitle && (
        <Typography fontSize={14} mb={3} textTransform={"uppercase"}>
          {overTitle}
        </Typography>
      )}
      {title && (
        <Typography fontSize={24} mb={!upDownLine ? 3 : 0}>
          {title}
        </Typography>
      )}
      {upDownLine && <CardUpDownPriceLine {...upDownLine} />}
      {titlePrice && <CardTitlePrice {...titlePrice} />}
      {dataRows && <CardDataRows {...dataRows} />}
      {graph && (
        <Box mb={3}>
          <ExplorerLineChart data={graph} />
        </Box>
      )}
      {progressBar && (
        <Box mb={3}>
          <ExplorerProgressBar {...progressBar} />
        </Box>
      )}
      {paragraph && <Typography>{paragraph}</Typography>}
    </CardContent>
  </Card>
);
