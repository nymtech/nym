import type { StaticImageData } from "next/image";

import stakeArticle from "@/../public/images/stake-article.webp";
import network from "@/../public/images/Network.webp";
import placeholder from "@/../public/images/placeholder.webp";

export type ImageName =
  | "stakeArticle"
  | "network"
  | "placeholder"

export type Images = {
  [key in ImageName]: StaticImageData;
};

export const images: Images = {
  stakeArticle,
  network,
  placeholder,
};
