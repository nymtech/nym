import Image from "next/image";
import { icons } from "@/utils/getIconByName";

export const TokenDark = () => {
  return <Image src={icons.tokenDark} alt="Token" width={20} height={20} />;
};
