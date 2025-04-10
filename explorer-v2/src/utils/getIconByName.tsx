import type { StaticImageData } from "next/image";

import arrowUpRight from "@/../public/icons/arrow-up-right.svg";
import arrow from "@/../public/icons/arrow-up-right.svg";
import chevronMenu from "@/../public/icons/chevronMenu.svg";
import copyFileDark from "@/../public/icons/copy-file-dark.svg";
import copyFile from "@/../public/icons/copy-file.svg";
import crossDark from "@/../public/icons/cross-dark.svg";
import cross from "@/../public/icons/cross.svg";
import discord from "@/../public/icons/discord.svg";
import document from "@/../public/icons/document.svg";
import download from "@/../public/icons/download.svg";
import elips from "@/../public/icons/elips.svg";
import elipseDark from "@/../public/icons/elipse-dark.svg";
import explorerCardDark from "@/../public/icons/explorer-card-dark.svg";
import explorerCard from "@/../public/icons/explorer-card.svg";
import gatewayDark from "@/../public/icons/gateway-dark.svg";
import gateway from "@/../public/icons/gateway.svg";
import github from "@/../public/icons/github.svg";
import nymLogoWhite from "@/../public/icons/nym-logo-white.svg";
import nymLogo from "@/../public/icons/nym-logo.svg";
import stakeCardDark from "@/../public/icons/stake-card-dark.svg";
import stakeCard from "@/../public/icons/stake-card.svg";
import telegram from "@/../public/icons/telegram.svg";
import tokenDark from "@/../public/icons/token-dark.svg";
import token from "@/../public/icons/token.svg";
import twitter from "@/../public/icons/twitter.svg";
import youtube from "@/../public/icons/youtube.svg";
import youTubeInverted from "@/../public/icons/youtubeInverted.svg";

export type IconName =
  | "nymLogo"
  | "nymLogoWhite"
  | "token"
  | "tokenDark"
  | "gateway"
  | "elips"
  | "elipseDark"
  | "cross"
  | "crossDark"
  | "copyFile"
  | "copyFileDark"
  | "chevronMenu"
  | "document"
  | "download"
  | "discord"
  | "github"
  | "telegram"
  | "twitter"
  | "youtube"
  | "youTubeInverted"
  | "arrowUpRight"
  | "arrow"
  | "stakeCard"
  | "explorerCard"
  | "explorerCardDark"
  | "stakeCardDark"
  | "gatewayDark";

export type Icons = {
  [key in IconName]: StaticImageData;
};

export const icons: Icons = {
  nymLogo,
  nymLogoWhite,
  token,
  tokenDark,
  gateway,
  elips,
  elipseDark,
  cross,
  crossDark,
  copyFile,
  copyFileDark,
  chevronMenu,
  document,
  download,
  discord,
  github,
  telegram,
  twitter,
  youtube,
  youTubeInverted,
  arrowUpRight,
  arrow,
  stakeCard,
  explorerCard,
  stakeCardDark,
  explorerCardDark,
  gatewayDark,
};
