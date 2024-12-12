"use client";
import CopyToClipboard from "@/components/copyToClipboard/CopyToClipboard";
import { Box, Button, Card, CardContent, Typography } from "@mui/material";
import Image from "next/image";
import { QRCodeCanvas } from "qrcode.react";
import type React from "react";
import type { FC } from "react";
import profileImagePlaceholder from "../../../public/profileImagePlaceholder.png";
import CardProfileCountry from "../countryFlag/CountryFlag";
import type { ICountryFlag } from "../countryFlag/CountryFlag";
// import { Remark42Comments } from "../comments";
import { NymTokenSVG } from "../icons/NymTokenSVG";
import { type ILineChartData, LineChart } from "../lineChart";
import {
  type IUpDownPriceIndicatorProps,
  UpDownPriceIndicator,
} from "../price/UpDownPriceIndicator";
import type { IDynamicProgressBarProps } from "../progressBars/EpochProgressBar";
import { StarRating } from "../starRating";

interface ICardTitlePriceProps {
  price: number;
  upDownLine: IUpDownPriceIndicatorProps;
}
const CardTitlePrice = (props: ICardTitlePriceProps): React.ReactNode => {
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
            display={"flex"}
            justifyContent={"space-between"}
            borderBottom={i < rows.length - 1 ? "1px solid #C3D7D7" : "none"}
            sx={{ pt: 1, pb: i < rows.length - 1 ? 1 : 0 }}
          >
            <Typography variant="h6" sx={{ color: "pine.600" }}>
              {row.key}
            </Typography>
            <Typography variant="h6" sx={{ color: "pine.600" }}>
              {row.value}
            </Typography>
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
    <Box display={"flex"} justifyContent={"flex-start"}>
      {url ? (
        <Image src={url} alt="linkedIn" width={80} height={80} />
      ) : (
        <Image
          src={profileImagePlaceholder}
          alt="profile picture"
          width={80}
          height={80}
        />
      )}
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
      paddingTop={1}
      paddingBottom={2}
      display={"flex"}
      flexDirection={"column"}
      gap={2}
      borderBottom={"1px solid #C3D7D7"}
    >
      <Typography variant="subtitle2" sx={{ color: "pine.600" }}>
        {title}
      </Typography>
      <Box display={"flex"} justifyContent={"space-between"}>
        <Typography
          variant="body4"
          maxWidth={"90%"}
          sx={{ wordWrap: "break-word", color: "pine.950" }}
        >
          {address}
        </Typography>

        <CopyToClipboard text={`Copy identity key ${address} to clipboard`} />
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
        padding={1}
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
        const RatingTitle = () => {
          switch (rating.numberOfStars) {
            case 1:
              return (
                <Typography variant="body4" sx={{ color: "pine.950" }}>
                  Bad
                </Typography>
              );
            case 2:
              return (
                <Typography variant="body4" sx={{ color: "pine.950" }}>
                  Bad
                </Typography>
              );
            case 3:
              return (
                <Typography variant="body4" sx={{ color: "pine.950" }}>
                  ok
                </Typography>
              );
            case 4:
              return (
                <Typography variant="body4" sx={{ color: "pine.950" }}>
                  Good
                </Typography>
              );
            default:
              return (
                <Typography variant="body4" sx={{ color: "pine.950" }}>
                  Excellent
                </Typography>
              );
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
            <Typography variant="subtitle2" sx={{ color: "pine.600" }}>
              {rating.title}
            </Typography>
            <Box display={"flex"} gap={1} alignItems={"center"}>
              <StarRating value={rating.numberOfStars} />
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
  profileCountry?: ICountryFlag;
  upDownLine?: IUpDownPriceIndicatorProps;
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
  <Card sx={{ height: "100%", borderRadius: "unset", padding: 1 }}>
    <CardContent
      sx={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
      }}
    >
      <Box>
        {overTitle && (
          <Typography
            variant="h5"
            sx={{ color: "pine.600", letterSpacing: 0.7 }}
          >
            {overTitle}
          </Typography>
        )}
        {profileImage && (
          <Box mt={3}>
            <CardProfileImage {...profileImage} />
          </Box>
        )}
        {title && (
          <Typography
            variant="h3"
            mt={3}
            sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
          >
            {title}
          </Typography>
        )}
        {upDownLine && (
          <Box>
            <UpDownPriceIndicator {...upDownLine} />
          </Box>
        )}
        {profileCountry && (
          <Box mt={3}>
            <CardProfileCountry {...profileCountry} />
          </Box>
        )}
        {titlePrice && (
          <Box mt={3}>
            <CardTitlePrice {...titlePrice} />
          </Box>
        )}
      </Box>
      {qrCode && (
        <Box mt={3}>
          <CardQRCode {...qrCode} />
        </Box>
      )}
      {nymAddress && (
        <Box mt={3}>
          <CardCopyAddress {...nymAddress} />
        </Box>
      )}
      {identityKey && (
        <Box mt={3}>
          <CardCopyAddress {...identityKey} />
        </Box>
      )}
      {dataRows && (
        <Box mt={3}>
          <CardDataRows {...dataRows} />
        </Box>
      )}
      {ratings && (
        <Box mt={3}>
          <CardRatings {...ratings} />
        </Box>
      )}
      {graph && (
        <Box mt={3}>
          <LineChart
            data={graph.data}
            color={graph.color}
            label={graph.label}
          />
        </Box>
      )}
      {progressBar && (
        <Box mt={3}>
          <DynamicProgressBar {...progressBar} />
        </Box>
      )}
      {paragraph && (
        <Typography variant="subtitle1" sx={{ color: "pine.950" }} mt={3}>
          {paragraph}
        </Typography>
      )}
      {/* {comments && (
        <Box mt={3}>
          <Remark42Comments />
        </Box>
      )} */}
      {stakeButton && (
        <Box mt={3}>
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
