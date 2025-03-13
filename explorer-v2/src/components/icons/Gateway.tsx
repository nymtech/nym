import Image from "next/image";
import { icons } from "@/utils/getIconByName";

const Gateway = () => {
  return (
    <Image src={icons.gateway} alt="Chevron Menu" width={84} height={84} />
  );
};

export default Gateway;
