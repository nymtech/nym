import { Card, CardContent, Typography, Box, IconButton } from "@mui/material";
import React, { FC, ReactElement, ReactEventHandler } from "react";
import { ExplorerLineChart, IExplorerLineChartData } from "./ExplorerLineChart";
import {
  ExplorerProgressBar,
  IExplorerProgressBarProps,
} from "./ExplorerProgressBar";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import { NymTokenSVG } from "../icons/NymTokenSVG";
import { CopyToClipboard } from "@nymproject/react/clipboard/CopyToClipboard";

import Image from "next/image";
import profileImagePlaceholder from "../../public/profileImagePlaceholder.png";
import Flag from "react-world-flags";

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
            borderBottom={i === 0 ? "1px solid #CAD6D7" : "none"}
          >
            <Typography>{row.key}</Typography>
            <Typography>{row.value}</Typography>
          </Box>
        );
      })}
    </Box>
  );
};
interface ICardProileImage {
  url?: string;
}
const CardProfileImage = (props: ICardProileImage) => {
  const { url } = props;
  return (
    <Box display={"flex"} justifyContent={"flex-start"} mb={3}>
      {url ? (
        <Image src={url} alt="linkedIn" width={80} height={80} />
      ) : (
        <Image
          src={profileImagePlaceholder}
          alt="linkedIn"
          width={80}
          height={80}
        />
      )}
    </Box>
  );
};

interface ICardProfileCountry {
  countryCode: string;
  countryName: string;
}

const CardProfileCountry = (props: ICardProfileCountry) => {
  const { countryCode, countryName } = props;
  return (
    <Box display={"flex"} justifyContent={"flex-start"} gap={2} mb={3}>
      <Flag code={countryCode} width="20" />
      <Typography textTransform={"uppercase"}>{countryName}</Typography>
    </Box>
  );
};

interface ICardCopyAddressProps {
  title: string;
  address: string;
}

const CardCopyAddress = (props: ICardCopyAddressProps) => {
  const { title, address } = props;
  return (
    <Box
      paddingTop={2}
      paddingBottom={2}
      display={"flex"}
      flexDirection={"column"}
      gap={2}
      borderBottom={"1px solid #CAD6D7"}
    >
      <Typography textTransform={"uppercase"}>{title}</Typography>
      <Box display={"flex"} justifyContent={"space-between"}>
        <Typography>{address}</Typography>

        <CopyToClipboard
          sx={{ mr: 0.5, color: "grey.400" }}
          smallIcons
          value={address}
          tooltip={`Copy identity key ${address} to clipboard`}
        />
      </Box>
    </Box>
  );
};

export type ContentCardProps = {
  overTitle?: string;
  profileImage?: ICardProileImage;
  title?: string | number;
  profileCountry?: ICardProfileCountry;
  upDownLine?: ICardUpDownPriceLineProps;
  titlePrice?: ICardTitlePriceProps;
  dataRows?: ICardDataRowsProps;
  graph?: { data: Array<IExplorerLineChartData>; color: string; label: string };
  progressBar?: IExplorerProgressBarProps;
  paragraph?: string;
  onClick?: ReactEventHandler;
  nymAddress?: ICardCopyAddressProps;
  identityKey?: ICardCopyAddressProps;
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
  profileImage,
  profileCountry,
  nymAddress,
  identityKey,
}) => (
  <Card onClick={onClick} sx={{ height: "100%" }}>
    <CardContent>
      {overTitle && (
        <Typography fontSize={14} mb={3} textTransform={"uppercase"}>
          {overTitle}
        </Typography>
      )}
      {profileImage && <CardProfileImage {...profileImage} />}
      {title && (
        <Typography fontSize={24} mb={3}>
          {title}
        </Typography>
      )}
      {profileCountry && <CardProfileCountry {...profileCountry} />}
      {upDownLine && <CardUpDownPriceLine {...upDownLine} />}
      {titlePrice && <CardTitlePrice {...titlePrice} />}
      {nymAddress && <CardCopyAddress {...nymAddress} />}
      {identityKey && <CardCopyAddress {...identityKey} />}
      {dataRows && <CardDataRows {...dataRows} />}
      {graph && (
        <Box mb={3}>
          <ExplorerLineChart
            data={graph.data}
            color={graph.color}
            label={graph.label}
          />
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
