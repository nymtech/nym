"use client";
import Image from "next/image";
import Discord from "../../../public/icons/discord.svg";
import GitHub from "../../../public/icons/github.svg";
// Icons
import Telegram from "../../../public/icons/telegram.svg";
import X from "../../../public/icons/twitter.svg";
import YouTube from "../../../public/icons/youtube.svg";
import YouTubeInverted from "../../../public/icons/youtubeInverted.svg";

export const icons: { [key: string]: React.ComponentType } = {
  telegram: Telegram,
  youtube: YouTube,
  discord: Discord,
  github: GitHub,
  twitter: X,
};

// import { useMainContext } from "@/context";
export const SocialIcon = ({ channel }: { channel: string }): JSX.Element => {
  // const { mode } = useMainContext();
  const modeType = "light";

  const iconSrc =
    modeType === "light" && channel.toLowerCase() === "youtube"
      ? YouTubeInverted
      : icons[channel.toLowerCase()];
  return <Image src={iconSrc} alt={channel} width={32} height={32} />;
};
