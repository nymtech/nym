"use client";
import { icons } from "@/utils/getIconByName";
import Image from "next/image";

// import { useMainContext } from "@/context";
export const SocialIcon = ({ channel }: { channel: string }): JSX.Element => {
  // const { mode } = useMainContext();
  const modeType = "light";

  // ✅ Normalize channel name & apply theme logic
  const normalizedChannel = channel.toLowerCase();
  const iconName =
    modeType === "light" && normalizedChannel === "youtube"
      ? "youTubeInverted"
      : (normalizedChannel as keyof typeof icons);

  // ✅ Get the icon source dynamically
  const iconSrc = icons[iconName]?.src || "/icons/default-icon.svg";

  return <Image src={iconSrc} alt={channel} width={32} height={32} />;
};
