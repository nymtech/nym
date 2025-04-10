"use client";
import { icons } from "@/utils/getIconByName";
import type { PaletteMode } from "@mui/material";
import { useLocalStorage } from "@uidotdev/usehooks";
import Image from "next/image";

const NymLogo = () => {
  const [mode] = useLocalStorage<PaletteMode>("mode", "dark");

  return (
    <Image
      src={mode === "dark" ? icons.nymLogoWhite : icons.nymLogo}
      alt="Nym Logo"
      width={90}
      height={25}
    />
  );
};
export default NymLogo;
