import { icons } from "@/utils/getIconByName";
import Image from "next/image";

export const TokenDark = () => {
  return <Image src={icons.tokenDark} alt="Token" width={20} height={20} />;
};
