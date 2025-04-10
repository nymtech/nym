import { icons } from "@/utils/getIconByName";
import Image from "next/image";

export const Token = () => {
  return <Image src={icons.token} alt="Token" width={20} height={20} />;
};
