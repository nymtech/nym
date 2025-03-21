import Image from "next/image";
import { icons } from "@/utils/getIconByName";

export const Token = () => {
  return <Image src={icons.token} alt="Token" width={20} height={20} />;
};
