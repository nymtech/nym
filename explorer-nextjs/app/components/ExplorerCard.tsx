import { Card, CardContent, Typography, Box, Button } from "@mui/material";
import React, { FC, ReactElement, ReactEventHandler, useEffect } from "react";
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
import { QRCodeCanvas } from "qrcode.react";
import StarIcon from "@mui/icons-material/Star";
import Script from "next/script";
import { useMainContext } from "../context/main";

declare global {
  interface Window {
    remark_config: {
      host: string;
      site_id: string;
      components: string[];
      max_shown_comments: number;
      theme: string;
      locale: string;
      show_email_subscription: boolean;
      simple_view: boolean;
      no_footer: boolean;
    };
    REMARK42: {
      createInstance: (config: typeof window.remark_config) => void;
      changeTheme: (theme: "light" | "dark") => void;
    };
  }
}

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

export interface ICardDataRowsProps {
  rows: Array<{ key: string; value: string }>;
}
export const CardDataRows = (props: ICardDataRowsProps): React.ReactNode => {
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
        mb={3}
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
    <Box mb={3}>
      {ratings.map((rating, i) => {
        const Stars = () => {
          const stars = [];
          for (let i = 0; i < rating.numberOfStars; i++) {
            stars.push(<StarIcon sx={{ color: "#14E76F" }} fontSize="small" />);
          }
          return stars;
        };
        const RatingTitle = () => {
          if (rating.numberOfStars === 1) {
            return <Typography>Bad</Typography>;
          } else if (rating.numberOfStars === 2) {
            return <Typography>Bad</Typography>;
          } else if (rating.numberOfStars === 3) {
            return <Typography>ok</Typography>;
          } else if (rating.numberOfStars === 4) {
            return <Typography>Good</Typography>;
          } else {
            return <Typography>Excellent</Typography>;
          }
        };
        return (
          <Box
            key={i}
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

const CardChat = () => {
  const { mode } = useMainContext();

  useEffect(() => {
    if (typeof window !== "undefined") {
      // Set Remark42 configuration on the window object
      window.remark_config = {
        host: "http://localhost:8081",
        site_id: "remark42",
        components: ["embed", "last-comments"],
        max_shown_comments: 100,
        theme: mode === "light" ? "light" : "dark",
        locale: "en",
        show_email_subscription: false,
        simple_view: true,
        no_footer: true,
      };

      // Dynamically load the Remark42 script if it doesn't exist
      if (!document.getElementById("remark42-script")) {
        const script = document.createElement("script");
        script.src = `${window.remark_config.host}/web/embed.js`;
        script.async = true;
        script.defer = true;
        script.id = "remark42-script";
        document.body.appendChild(script);
      } else if (window.REMARK42) {
        // Re-initialize if the script is already loaded
        window.REMARK42.createInstance(window.remark_config);
      }
    }
  }, []);

  // React to mode changes and update Remark42 theme
  useEffect(() => {
    if (window.REMARK42 && window.REMARK42.changeTheme) {
      window.REMARK42.changeTheme(mode === "dark" ? "dark" : "light");
    }
  }, [mode]);

  return (
    <Box>
      <div id="remark42" className="remark"></div>
      <Script
        id="remark-init"
        strategy="afterInteractive"
        dangerouslySetInnerHTML={{
          __html: `
        if (window.REMARK42) {
          window.REMARK42.createInstance(window.remark_config);
        }
      `,
        }}
      />
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
  qrCode?: ICardQRCodeProps;
  ratings?: ICardRatingsProps;
  chat?: boolean;
  button?: {
    onClick: () => void;
    label: string;
  };
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
  qrCode,
  ratings,
  chat,
  button,
}) => (
  <Card onClick={onClick} sx={{ height: "100%", borderRadius: "unset" }}>
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
      {qrCode && <CardQRCode {...qrCode} />}
      {nymAddress && <CardCopyAddress {...nymAddress} />}
      {identityKey && <CardCopyAddress {...identityKey} />}
      {dataRows && <CardDataRows {...dataRows} />}
      {ratings && <CardRatings {...ratings} />}
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
      {chat && <CardChat />}
      {button && (
        <Button onClick={button.onClick} variant="contained">
          {button.label}
        </Button>
      )}
    </CardContent>
  </Card>
);
