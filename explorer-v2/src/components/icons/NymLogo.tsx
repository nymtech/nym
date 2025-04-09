"use client";
import Image from "next/image";
import { icons } from "@/utils/getIconByName";
import { PaletteMode } from "@mui/material";
import { useLocalStorage } from "@uidotdev/usehooks";

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
