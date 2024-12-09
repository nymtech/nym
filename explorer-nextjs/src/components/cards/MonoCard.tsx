"use client";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import StarIcon from "@mui/icons-material/Star";
import { Box, Button, Card, CardContent, Typography } from "@mui/material";
import { CopyToClipboard } from "@nymproject/react/clipboard/CopyToClipboard";
import Image from "next/image";
import { QRCodeCanvas } from "qrcode.react";
import type React from "react";
import type { FC, ReactElement } from "react";
import Flag from "react-world-flags";
import profileImagePlaceholder from "../../../public/profileImagePlaceholder.png";
// import { Remark42Comments } from "../comments";
import { NymTokenSVG } from "../icons/NymTokenSVG";
import { type ILineChartData, LineChart } from "../lineChart";
import {
  DynamicProgressBar,
  type IDynamicProgressBarProps,
} from "../progressBars/DynamicProgressBar";

interface ICardUpDownPriceLineProps {
  percentage: number;
  numberWentUp: boolean;
}
const CardUpDownPriceLine = (
  props: ICardUpDownPriceLineProps,
): ReactElement => {
  const { percentage, numberWentUp } = props;
  return (
    <Box display={"flex"} alignItems={"center"}>
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
        <Box display={"flex"} gap={1} alignItems={"center"}>
          <NymTokenSVG />
          <Typography>NYM</Typography>
        </Box>
        <Typography>${price}</Typography>
      </Box>
      <CardUpDownPriceLine {...upDownLine} />
    </Box>
  );
};

export interface ICardDataRowsProps {
  rows: Array<{ key: string; value: string }>;
}
export const CardDataRows = (props: ICardDataRowsProps): React.ReactNode => {
  const { rows } = props;

  return (
    <Box>
      {rows.map((row, i) => {
        return (
          <Box
            key={row.key}
            paddingTop={2}
            paddingBottom={2}
            display={"flex"}
            justifyContent={"space-between"}
            borderBottom={i === 0 ? "1px solid #C3D7D7" : "none"}
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
    <Box display={"flex"} justifyContent={"flex-start"} gap={2}>
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
      borderBottom={"1px solid #C3D7D7"}
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

interface ICardQRCodeProps {
  url: string;
}

const CardQRCode = (props: ICardQRCodeProps) => {
  const { url } = props;
  return (
    <Box display={"flex"} justifyContent={"flex-start"}>
      <Box
        padding={2}
        border={"1px solid #C3D7D7"}
        display={"block"}
        width={"unset"}
      >
        <QRCodeCanvas value={url} />
      </Box>
    </Box>
  );
};

interface ICardRatingsProps {
  ratings: Array<{ title: string; numberOfStars: number }>;
}

const CardRatings = (props: ICardRatingsProps) => {
  const { ratings } = props;

  return (
    <Box>
      {ratings.map((rating, i) => {
        const Stars = () => {
          const stars = [];
          for (let i = 0; i < rating.numberOfStars; i++) {
            stars.push(
              <StarIcon sx={{ color: "#14E76F" }} fontSize="small" key={i} />,
            );
          }
          return stars;
        };
        const RatingTitle = () => {
          switch (rating.numberOfStars) {
            case 1:
            case 2:
              return <Typography>Bad</Typography>;
            case 3:
              return <Typography>ok</Typography>;
            case 4:
              return <Typography>Good</Typography>;
            default:
              return <Typography>Excellent</Typography>;
          }
        };
        return (
          <Box
            key={rating.title}
            paddingTop={2}
            paddingBottom={2}
            display={"flex"}
            justifyContent={"space-between"}
            borderBottom={i < ratings.length - 1 ? "1px solid #C3D7D7" : "none"}
          >
            <Typography>{rating.title}</Typography>
            <Box display={"flex"} gap={1} alignItems={"center"}>
              <Stars />
              <RatingTitle />
            </Box>
          </Box>
        );
      })}
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
  graph?: { data: Array<ILineChartData>; color: string; label: string };
  progressBar?: IDynamicProgressBarProps;
  paragraph?: string;
  nymAddress?: ICardCopyAddressProps;
  identityKey?: ICardCopyAddressProps;
  qrCode?: ICardQRCodeProps;
  ratings?: ICardRatingsProps;
  comments?: boolean;
  stakeButton?: {
    label: string;
    identityKey: string;
  };
};

export const MonoCard: FC<ContentCardProps> = ({
  title,
  titlePrice,
  overTitle,
  upDownLine,
  dataRows,
  graph,
  progressBar,
  paragraph,
  profileImage,
  profileCountry,
  nymAddress,
  identityKey,
  qrCode,
  ratings,
  // comments,
  stakeButton,
}) => (
  <Card sx={{ height: "100%", borderRadius: "unset", padding: 3 }}>
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
      {profileCountry && (
        <Box mb={3}>
          <CardProfileCountry {...profileCountry} />
        </Box>
      )}
      {upDownLine && (
        <Box mb={3}>
          <CardUpDownPriceLine {...upDownLine} />
        </Box>
      )}
      {titlePrice && <CardTitlePrice {...titlePrice} />}
      {qrCode && (
        <Box mb={3}>
          <CardQRCode {...qrCode} />
        </Box>
      )}
      {nymAddress && (
        <Box mb={3}>
          <CardCopyAddress {...nymAddress} />
        </Box>
      )}
      {identityKey && (
        <Box mb={3}>
          <CardCopyAddress {...identityKey} />
        </Box>
      )}
      {dataRows && (
        <Box mb={3}>
          <CardDataRows {...dataRows} />
        </Box>
      )}
      {ratings && (
        <Box mb={3}>
          <CardRatings {...ratings} />
        </Box>
      )}
      {graph && (
        <Box mb={3}>
          <LineChart
            data={graph.data}
            color={graph.color}
            label={graph.label}
          />
        </Box>
      )}
      {progressBar && (
        <Box mb={3}>
          <DynamicProgressBar {...progressBar} />
        </Box>
      )}
      {paragraph && <Typography>{paragraph}</Typography>}
      {/* {comments && (
        <Box mb={3}>
          <Remark42Comments />
        </Box>
      )} */}
      {stakeButton && (
        <Box mb={3}>
          <Button
            onClick={() =>
              console.log(
                "stakeButton.identityKey :>> ",
                stakeButton.identityKey,
              )
            }
            variant="contained"
          >
            {stakeButton.label}
          </Button>
        </Box>
      )}
    </CardContent>
  </Card>
);
