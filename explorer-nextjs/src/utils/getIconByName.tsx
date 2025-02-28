import type { StaticImageData } from "next/image";

import nymLogo from "@/../public/icons/nym-logo.svg";
import token from "@/../public/icons/token.svg";
import gateway from "@/../public/icons/gateway.svg";
import elips from "@/../public/icons/elips.svg";
import cross from "@/../public/icons/cross.svg";
import copyFile from "@/../public/icons/copy-file.svg";
import chevronMenu from "@/../public/icons/chevronMenu.svg"
import document from "@/../public/icons/document.svg"
import download from "@/../public/icons/download.svg"
import discord from "@/../public/icons/discord.svg"
import github from "@/../public/icons/github.svg"
import telegram from "@/../public/icons/telegram.svg"
import twitter from "@/../public/icons/twitter.svg"
import youtube from "@/../public/icons/youtube.svg"
import youTubeInverted from "@/../public/icons/youtubeInverted.svg"
import arrowUpRight from "@/../public/icons/arrow-up-right.svg"
import arrow from "@/../public/icons/arrow-up-right.svg"
import explorerCard from "@/../public/icons/explorer-card.svg"
import stakeCard from "@/../public/icons/stake-card.svg"

export type IconName =
  | "nymLogo"
  | "token"
  | "gateway"
  | "elips"
  | "cross"
  | "copyFile"
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

export type Icons = {
  [key in IconName]: StaticImageData;
};

export const icons: Icons = {
  nymLogo,
  token,
  gateway,
  elips,
  cross,
  copyFile,
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
  explorerCard

};
